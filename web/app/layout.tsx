import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
  display: "swap",
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
  display: "swap",
});

const siteUrl = "https://supertype.app";

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: "Supertype — Your voice, everywhere you type.",
    template: "%s — Supertype",
  },
  description:
    "Privacy-first, local voice-to-text for macOS. Hold one shortcut, speak naturally, and your words appear where you type. No cloud required. Audio stays on your Mac.",
  keywords: ["macOS", "voice to text", "dictation", "local", "privacy", "whisper", "parakeet", "speech recognition"],
  authors: [{ name: "Supertype" }],
  creator: "Supertype",
  openGraph: {
    type: "website",
    locale: "en_US",
    url: siteUrl,
    title: "Supertype — Your voice, everywhere you type.",
    description:
      "Hold one shortcut, speak, and your words appear where you type. 100% local transcription for macOS. Your voice stays on your Mac.",
    siteName: "Supertype",
  },
  twitter: {
    card: "summary_large_image",
    title: "Supertype — Your voice, everywhere you type.",
    description: "Local voice-to-text for macOS. Private, fast, and offline after one download.",
  },
  icons: { icon: "/favicon.svg" },
  alternates: { canonical: siteUrl },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}>
      <body className="min-h-full flex flex-col bg-[var(--bg)] text-[var(--fg)]">
        <a
          href="#main"
          className="sr-only focus:not-sr-only focus:absolute focus:top-3 focus:left-3 focus:z-[100] focus:px-3 focus:py-2 focus:bg-[var(--fg)] focus:text-[var(--bg)] focus:rounded-md focus:text-sm"
        >
          Skip to content
        </a>
        {children}
      </body>
    </html>
  );
}
