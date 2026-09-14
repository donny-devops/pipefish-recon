import type { Metadata } from "next";
import type { ReactNode } from "react";
import Link from "next/link";
import "./globals.css";

export const metadata: Metadata = {
  title: "PipeFish RECON operator",
  description: "Localhost operator surface for the PipeFish RECON Agentic OS kernel",
};

export default function RootLayout({
  children,
}: {
  children: ReactNode;
}) {
  return (
    <html lang="en">
      <body>
        <div className="shell">
          <header className="topbar">
            <div className="brand">
              <strong>PIPEFISH RECON</strong>
              <span>Operator console</span>
            </div>
            <nav>
              <Link href="/">Status</Link>
              <Link href="/events">Events</Link>
              <Link href="/policy">Policy</Link>
              <Link href="/agents">Agents</Link>
            </nav>
          </header>
          {children}
        </div>
      </body>
    </html>
  );
}
