use anyhow::Result;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch, Semaphore};
use tokio_rustls::TlsConnector;

use crate::cache::CacheManager;
use crate::models::{ServerInfo, TestResult, TestStatus};

#[derive(Debug)]
struct NoCertVerifier;

impl ServerCertVerifier for NoCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub struct BulkScanner {
    servers: Vec<ServerInfo>,
    cache_manager: CacheManager,
    concurrency: usize,
    port: u16,
    timeout: Duration,
    mode: String,
    paused_tx: watch::Sender<bool>,
    paused_rx: watch::Receiver<bool>,
    stopped: Arc<AtomicBool>,
    tls_connector: Option<TlsConnector>,
}

impl BulkScanner {
    pub fn new(
        servers: Vec<ServerInfo>,
        cache_manager: CacheManager,
        concurrency: usize,
        port: u16,
        timeout_secs: f64,
        mode: &str,
    ) -> Self {
        let concurrency = concurrency.max(1);
        let timeout = Duration::from_secs_f64(timeout_secs.max(0.1));
        let (paused_tx, paused_rx) = watch::channel(false);
        let stopped = Arc::new(AtomicBool::new(false));

        let mut tls_connector = None;
        if mode.eq_ignore_ascii_case("tls") {
            let _ = rustls::crypto::ring::default_provider().install_default();
            let mut config = ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoCertVerifier))
                .with_no_client_auth();
            config.enable_sni = false;
            tls_connector = Some(TlsConnector::from(Arc::new(config)));
        }

        Self {
            servers,
            cache_manager,
            concurrency,
            port,
            timeout,
            mode: mode.to_lowercase(),
            paused_tx,
            paused_rx,
            stopped,
            tls_connector,
        }
    }

    pub fn is_paused(&self) -> bool {
        *self.paused_rx.borrow()
    }

    pub fn toggle_pause(&self) -> bool {
        let next = !self.is_paused();
        let _ = self.paused_tx.send(next);
        next
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = self.paused_tx.send(false); // Unpause so workers can exit
    }

    async fn probe_single(
        server: &ServerInfo,
        port: u16,
        timeout: Duration,
        mode: &str,
        tls_connector: Option<&TlsConnector>,
        mut paused_rx: watch::Receiver<bool>,
        stopped: &AtomicBool,
    ) -> TestResult {
        // Wait if paused
        while *paused_rx.borrow() {
            if stopped.load(Ordering::Relaxed) {
                break;
            }
            if paused_rx.changed().await.is_err() {
                break;
            }
        }

        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        if stopped.load(Ordering::Relaxed) {
            return TestResult {
                server_id: server.id,
                name: server.name.clone(),
                hostname: server.hostname.clone(),
                station: server.station.clone(),
                country_name: server.country_name.clone(),
                country_code: server.country_code.clone(),
                city: server.city.clone(),
                load: server.load,
                port,
                status: TestStatus::Pending,
                latency_ms: None,
                tested_at: now_epoch,
                is_cached: false,
                features: server.feature_tags(),
                error_msg: Some("Cancelled".to_string()),
            };
        }

        // Parse target address
        let target_ip: Option<IpAddr> = server.station.parse().ok();
        let socket_addr = match target_ip {
            Some(ip) => Some(SocketAddr::new(ip, port)),
            None => None,
        };

        let t0 = Instant::now();

        let probe_fut = async {
            match socket_addr {
                Some(addr) => TcpStream::connect(addr).await,
                None => {
                    let addr_str = format!("{}:{}", server.station, port);
                    TcpStream::connect(addr_str).await
                }
            }
        };

        let res = tokio::time::timeout(timeout, probe_fut).await;

        let (status, latency_ms, error_msg) = match res {
            Ok(Ok(stream)) => {
                if mode == "tls" {
                    if let Some(connector) = tls_connector {
                        let domain_str = if !server.hostname.is_empty() {
                            server.hostname.clone()
                        } else {
                            server.station.clone()
                        };

                        let server_name = match ServerName::try_from(domain_str.as_str()) {
                            Ok(name) => name.to_owned(),
                            Err(_) => {
                                // Fallback to a dummy IP/DNS name
                                ServerName::try_from("nordvpn.com")
                                    .unwrap()
                                    .to_owned()
                            }
                        };

                        let tls_fut = connector.connect(server_name, stream);
                        match tokio::time::timeout(timeout, tls_fut).await {
                            Ok(Ok(_tls_stream)) => {
                                let lat = t0.elapsed().as_secs_f64() * 1000.0;
                                (TestStatus::Online, Some((lat * 100.0).round() / 100.0), None)
                            }
                            Ok(Err(e)) => (
                                TestStatus::Error,
                                None,
                                Some(format!("TLS handshake failed: {}", e)),
                            ),
                            Err(_) => (
                                TestStatus::Timeout,
                                None,
                                Some(format!("TLS handshake timed out (>{:.1}s)", timeout.as_secs_f64())),
                            ),
                        }
                    } else {
                        let lat = t0.elapsed().as_secs_f64() * 1000.0;
                        (TestStatus::Online, Some((lat * 100.0).round() / 100.0), None)
                    }
                } else {
                    let lat = t0.elapsed().as_secs_f64() * 1000.0;
                    (TestStatus::Online, Some((lat * 100.0).round() / 100.0), None)
                }
            }
            Ok(Err(e)) => {
                let err_str = e.to_string();
                let os_err = e.raw_os_error();

                let (st, msg) = match e.kind() {
                    std::io::ErrorKind::ConnectionRefused => {
                        (TestStatus::Refused, "Connection refused".to_string())
                    }
                    std::io::ErrorKind::TimedOut => (
                        TestStatus::Timeout,
                        format!("Timed out / Dropped by network ({})", err_str),
                    ),
                    _ => {
                        let lower = err_str.to_lowercase();
                        if lower.contains("unreachable") {
                            (TestStatus::Unreachable, err_str)
                        } else if os_err == Some(10060)
                            || os_err == Some(121)
                            || lower.contains("semaphore timeout")
                            || lower.contains("timed out")
                        {
                            (
                                TestStatus::Timeout,
                                format!("Timed out / Dropped by network ({})", err_str),
                            )
                        } else if os_err == Some(10061) {
                            (TestStatus::Refused, "Connection refused".to_string())
                        } else if os_err == Some(10065) || os_err == Some(10051) {
                            (TestStatus::Unreachable, err_str)
                        } else {
                            (TestStatus::Error, err_str)
                        }
                    }
                };
                (st, None, Some(msg))
            }
            Err(_) => (
                TestStatus::Timeout,
                None,
                Some(format!("Timed out (>{:.1}s)", timeout.as_secs_f64())),
            ),
        };

        TestResult {
            server_id: server.id,
            name: server.name.clone(),
            hostname: server.hostname.clone(),
            station: server.station.clone(),
            country_name: server.country_name.clone(),
            country_code: server.country_code.clone(),
            city: server.city.clone(),
            load: server.load,
            port,
            status,
            latency_ms,
            tested_at: now_epoch,
            is_cached: false,
            features: server.feature_tags(),
            error_msg,
        }
    }

    pub async fn run<F>(&self, on_result: F)
    where
        F: Fn(TestResult) + Send + Sync + 'static,
    {
        self.stopped.store(false, Ordering::SeqCst);
        let _ = self.paused_tx.send(false);

        let (db_tx, mut db_rx) = mpsc::channel::<Option<TestResult>>(2048);
        let cache_mgr = self.cache_manager.clone();
        let stopped_clone = self.stopped.clone();

        // Background batch DB writer
        let db_writer_task = tokio::spawn(async move {
            let mut batch = Vec::with_capacity(100);
            let mut interval = tokio::time::interval(Duration::from_millis(100));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            let _ = cache_mgr.save_batch(&batch);
                            batch.clear();
                        }
                        if stopped_clone.load(Ordering::Relaxed) && batch.is_empty() {
                            break;
                        }
                    }
                    item = db_rx.recv() => {
                        match item {
                            Some(Some(res)) => {
                                batch.push(res);
                                if batch.len() >= 100 {
                                    let _ = cache_mgr.save_batch(&batch);
                                    batch.clear();
                                }
                            }
                            Some(None) | None => {
                                // Sentinel or channel closed
                                break;
                            }
                        }
                    }
                }
            }

            if !batch.is_empty() {
                let _ = cache_mgr.save_batch(&batch);
            }
        });

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let on_result = Arc::new(on_result);

        let mut join_handles = Vec::with_capacity(self.servers.len());

        for server in &self.servers {
            if self.stopped.load(Ordering::Relaxed) {
                break;
            }

            let sem = semaphore.clone();
            let s = server.clone();
            let port = self.port;
            let timeout = self.timeout;
            let mode = self.mode.clone();
            let tls_connector = self.tls_connector.clone();
            let paused_rx = self.paused_rx.clone();
            let stopped = self.stopped.clone();
            let db_tx = db_tx.clone();
            let on_result = on_result.clone();

            let handle = tokio::spawn(async move {
                let _permit = match sem.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => return,
                };

                if stopped.load(Ordering::Relaxed) {
                    return;
                }

                let result = Self::probe_single(
                    &s,
                    port,
                    timeout,
                    &mode,
                    tls_connector.as_ref(),
                    paused_rx,
                    &stopped,
                )
                .await;

                if !stopped.load(Ordering::Relaxed) {
                    let _ = db_tx.send(Some(result.clone())).await;
                    on_result(result);
                }
            });

            join_handles.push(handle);
        }

        // Await all probes
        for handle in join_handles {
            let _ = handle.await;
        }

        // Flush and terminate DB writer
        let _ = db_tx.send(None).await;
        let _ = db_writer_task.await;
    }
}
