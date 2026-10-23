#!/usr/bin/env python3
"""
NordVPN OpenVPN Proxy Config Generator
Creates custom .ovpn profiles that route NordVPN connections through a local SOCKS5 or HTTP proxy
(e.g., V2Ray, Xray, Nekoray, Hiddify, Sing-box, or SSH tunnel) to bypass national DPI / IP filtering.
"""

import argparse
import json
import sys
from pathlib import Path

SERVERS_CACHE = Path(".cache/nord_servers_metadata.json")

# Standard NordVPN Root CA & TLS-Auth / TLS-Crypt keys (universal across all NordVPN servers)
NORD_CA_CERT = """-----BEGIN CERTIFICATE-----
MIIFCjCCAvKgAwIBAgIBATANBgkqhkiG9w0BAQ0FADA5MQswCQYDVQQGEwJQQTEQ
MA4GA1UEChMHTm9yZFZQTjEYMBYGA1UEAxMPTm9yZFZQTiBSb290IENBMB4XDTE2
MDEwMTAwMDAwMFoXDTM1MTIzMTIzNTk1OVowOTELMAkGA1UEBhMCUEExEDAOBgNV
BAoTB05vcmRWUE4xGDAWBgNVBAMTD05vcmRWUE4gUm9vdCBDQTCCAiIwDQYJKoZI
hvcNAQEBBQADggIPADCCAgoCggIBAMkr/BYhyo0F2upsIMXwC6QvkZps3NN2/eQF
kfQIS1gql0aejsKsEnmY0Kaon8uZCTXPsRH1gQNgg5D2gixdd1mJUvV3dE3y9FJr
XMoDkXdCGBodvKJyU6lcfEVF6/UxHcbBguZK9UtRHS9eJYm3rpL/5huQMCppX7kU
eQ8dpCwd3iKITqwd1ZudDqsWaU0vqzC2H55IyaZ/5/TnCk31Q1UP6BksbbuRcwOV
skEDsm6YoWDnn/IIzGOYnFJRzQH5jTz3j1QBvRIuQuBuvUkfhx1FEwhwZigrcxXu
MP+QgM54kezgziJUaZcOM2zF3lvrwMvXDMfNeIoJABv9ljw969xQ8czQCU5lMVmA
37ltv5Ec9U5hZuwk/9QO1Z+d/r6Jx0mlurS8gnCAKJgwa3kyZw6e4FZ8mYL4vpRR
hPdvRTWCMJkeB4yBHyhxUmTRgJHm6YR3D6hcFAc9cQcTEl/I60tMdz33G6m0O42s
Qt/+AR3YCY/RusWVBJB/qNS94EtNtj8iaebCQW1jHAhvGmFILVR9lzD0EzWKHkvy
WEjmUVRgCDd6Ne3eFRNS73gdv/C3l5boYySeu4exkEYVxVRn8DhCxs0MnkMHWFK6
MyzXCCn+JnWFDYPfDKHvpff/kLDobtPBf+Lbch5wQy9quY27xaj0XwLyjOltpiST
LWae/Q4vAgMBAAGjHTAbMAwGA1UdEwQFMAMBAf8wCwYDVR0PBAQDAgEGMA0GCSqG
SIb3DQEBDQUAA4ICAQC9fUL2sZPxIN2mD32VeNySTgZlCEdVmlq471o/bDMP4B8g
nQesFRtXY2ZCjs50Jm73B2LViL9qlREmI6vE5IC8IsRBJSV4ce1WYxyXro5rmVg/
k6a10rlsbK/eg//GHoJxDdXDOokLUSnxt7gk3QKpX6eCdh67p0PuWm/7WUJQxH2S
DxsT9vB/iZriTIEe/ILoOQF0Aqp7AgNCcLcLAmbxXQkXYCCSB35Vp06u+eTWjG0/
pyS5V14stGtw+fA0DJp5ZJV4eqJ5LqxMlYvEZ/qKTEdoCeaXv2QEmN6dVqjDoTAo
k0t5u4YRXzEVCfXAC3ocplNdtCA72wjFJcSbfif4BSC8bDACTXtnPC7nD0VndZLp
+RiNLeiENhk0oTC+UVdSc+n2nJOzkCK0vYu0Ads4JGIB7g8IB3z2t9ICmsWrgnhd
NdcOe15BincrGA8avQ1cWXsfIKEjbrnEuEk9b5jel6NfHtPKoHc9mDpRdNPISeVa
wDBM1mJChneHt59Nh8Gah74+TM1jBsw4fhJPvoc7Atcg740JErb904mZfkIEmojC
VPhBHVQ9LHBAdM8qFI2kRK0IynOmAZhexlP/aT/kpEsEPyaZQlnBn3An1CRz8h0S
PApL8PytggYKeQmRhl499+6jLxcZ2IegLfqq41dzIjwHwTMplg+1pKIOVojpWA==
-----END CERTIFICATE-----"""

NORD_TLS_AUTH_KEY = """-----BEGIN OpenVPN Static key V1-----
e685bdaf659a25a200e2b9e39e51ff03
0fc72cf1ce07232bd8b2be5e6c670143
f51e937e670eee09d4f2ea5a6e4e6996
5db852c275351b86fc4ca892d78ae002
d6f70d029bd79c4d1c26cf14e9588033
cf639f8a74809f29f72b9d58f9b8f5fe
fc7938eade40e9fed6cb92184abb2cc1
0eb1a296df243b251df0643d53724cdb
5a92a1d6cb817804c4a9319b57d53be5
80815bcfcb2df55018cc83fc43bc7ff8
2d51f9b88364776ee9d12fc85cc7ea5b
9741c4f598c485316db066d52db4540e
212e1518a9bd4828219e24b20d88f598
a196c9de96012090e333519ae18d3509
9427e7b372d348d352dc4c85e18cd4b9
3f8a56ddb2e64eb67adfc9b337157ff4
-----END OpenVPN Static key V1-----"""


def load_servers():
    if not SERVERS_CACHE.exists():
        print(f"Server cache not found at {SERVERS_CACHE}. Please run main.py once to fetch server list.")
        sys.exit(1)
    with open(SERVERS_CACHE, "r", encoding="utf-8") as f:
        return json.load(f)


def find_best_server(servers, country_code="DE"):
    country_code = country_code.upper()
    candidates = []
    for s in servers:
        locs = s.get("locations", [])
        if locs:
            c = locs[0].get("country", {}).get("code", "")
            if c.upper() == country_code:
                candidates.append(s)
    if not candidates:
        print(f"No servers found for country: {country_code}")
        return None
    # Sort by lowest load
    candidates.sort(key=lambda x: x.get("load", 100))
    return candidates[0]


def build_ovpn_profile(server_info: dict, proto="tcp", port=443, proxy_type="socks", proxy_host="127.0.0.1", proxy_port=10808):
    hostname = server_info.get("hostname")
    station_ip = server_info.get("station")

    lines = [
        "client",
        "dev tun",
        f"proto {proto.lower()}",
        f"remote {station_ip} {port}",
        f"remote {hostname} {port}",
        "resolv-retry infinite",
        "nobind",
        "persist-key",
        "persist-tun",
        "fast-io",
        "remote-cert-tls server",
        "auth-user-pass",
        "verb 3",
        "pull",
        "cipher AES-256-CBC",
        "auth SHA512",
        f"verify-x509-name CN={hostname}",
        "key-direction 1",
    ]

    # Proxy configuration (bypasses ISP DPI / firewall)
    if proxy_type.lower() == "socks":
        lines.insert(0, f"# Tunnel through local SOCKS5 proxy to bypass national DPI/filtering")
        lines.insert(1, f"socks-proxy {proxy_host} {proxy_port}")
        lines.insert(2, f"socks-proxy-retry")
    elif proxy_type.lower() == "http":
        lines.insert(0, f"# Tunnel through local HTTP proxy to bypass national DPI/filtering")
        lines.insert(1, f"http-proxy {proxy_host} {proxy_port}")
        lines.insert(2, f"http-proxy-retry")

    content = "\n".join(lines) + "\n"
    content += f"<ca>\n{NORD_CA_CERT}\n</ca>\n"
    content += f"<tls-auth>\n{NORD_TLS_AUTH_KEY}\n</tls-auth>\n"
    return content


def main():
    parser = argparse.ArgumentParser(description="Generate NordVPN OpenVPN profile routed through a local anti-censorship proxy.")
    parser.add_argument("-c", "--country", default="DE", help="Target country code (e.g., DE, NL, US, GB, TR, AE). Default: DE.")
    parser.add_argument("-s", "--server", default=None, help="Explicit server hostname (e.g. de1055.nordvpn.com). Overrides --country.")
    parser.add_argument("--port", type=int, default=443, help="NordVPN server port: 443, 1231, 1232 (default: 443).")
    parser.add_argument("--proto", choices=["tcp", "udp"], default="tcp", help="Protocol: tcp (recommended for proxies) or udp.")
    parser.add_argument("--proxy-type", choices=["socks", "http"], default="socks", help="Local proxy type (socks or http). Default: socks.")
    parser.add_argument("--proxy-host", default="127.0.0.1", help="Local proxy host (default: 127.0.0.1).")
    parser.add_argument("--proxy-port", type=int, default=10808, help="Local proxy port (default: 10808 for V2rayN/Nekoray, 10809 for HTTP, 2080 for Hiddify).")
    parser.add_argument("-o", "--output", default=None, help="Output .ovpn file path.")

    args = parser.parse_args()
    servers = load_servers()

    server_info = None
    if args.server:
        for s in servers:
            if s.get("hostname", "").lower() == args.server.lower():
                server_info = s
                break
        if not server_info:
            server_info = {"hostname": args.server, "station": args.server, "name": args.server, "load": 0}
    else:
        server_info = find_best_server(servers, country_code=args.country)
        if not server_info:
            sys.exit(1)

    print(f"Generating profile for: {server_info['name']} ({server_info['hostname']}, IP: {server_info['station']}, Load: {server_info.get('load', 0)}%)")
    content = build_ovpn_profile(
        server_info=server_info,
        proto=args.proto,
        port=args.port,
        proxy_type=args.proxy_type,
        proxy_host=args.proxy_host,
        proxy_port=args.proxy_port,
    )

    out_name = args.output or f"{server_info['hostname']}_proxied.ovpn"
    out_path = Path(out_name)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(content)

    print(f"\n[SUCCESS] Generated: {out_path.resolve()}")
    print("\nNext Steps:")
    print("1. Ensure your local proxy is running (e.g. V2rayN / Nekoray / Sing-Box / Hiddify on port 10808).")
    print("2. Open 'OpenVPN Connect' (installed at 'C:\\Program Files\\OpenVPN Connect\\OpenVPNConnect.exe').")
    print(f"3. Import '{out_path.name}' into OpenVPN Connect (Upload File).")
    print("4. Enter your NordVPN manual service credentials (get them at https://my.nordaccount.com -> NordVPN -> Set up manually).")
    print("5. Click Connect!")


if __name__ == "__main__":
    main()
