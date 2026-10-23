import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatTimeAgo(timestampSeconds: number | null | undefined): string {
  if (!timestampSeconds || timestampSeconds <= 0) {
    return "Never";
  }

  const now = Math.floor(Date.now() / 1000);
  const diff = Math.max(0, now - timestampSeconds);

  if (diff < 10) return "just now";
  if (diff < 60) return `${diff} seconds ago`;
  
  const minutes = Math.floor(diff / 60);
  if (minutes === 1) return "1 minute ago";
  if (minutes < 60) return `${minutes} minutes ago`;

  const hours = Math.floor(minutes / 60);
  if (hours === 1) return "1 hour ago";
  if (hours < 24) return `${hours} hours ago`;

  const days = Math.floor(hours / 24);
  if (days === 1) return "1 day ago";
  if (days < 30) return `${days} days ago`;

  const months = Math.floor(days / 30);
  if (months === 1) return "1 month ago";
  return `${months} months ago`;
}
