import type { Metadata } from "next";
import { Space_Grotesk } from "next/font/google";
import { GeistSans } from "geist/font/sans";
import { GeistMono } from "geist/font/mono";
import "./globals.css";
import { Nav } from "@/components/nav";
import { Footer } from "@/components/footer";
import { Cosmos } from "@/components/cosmos";

const display = Space_Grotesk({
  variable: "--font-display",
  subsets: ["latin"],
  weight: ["500", "600", "700"],
});

const SITE = "https://redstart-lang.com";
const DESCRIPTION =
  "Redstart unifies schema, manifest, and mappings into one typed language for The Graph subgraphs — and transpiles to AssemblyScript the canonical toolchain compiles unmodified.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE),
  title: {
    default: "Redstart — one language for The Graph subgraphs",
    template: "%s · Redstart",
  },
  description: DESCRIPTION,
  openGraph: {
    title: "Redstart — one language for The Graph subgraphs",
    description: DESCRIPTION,
    url: SITE,
    siteName: "Redstart",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Redstart",
    description: DESCRIPTION,
  },
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html
      lang="en"
      className={`${GeistSans.variable} ${GeistMono.variable} ${display.variable} h-full antialiased`}
    >
      <body className="relative flex min-h-full flex-col bg-bg text-text">
        <Cosmos />
        <Nav />
        <main className="relative flex-1">{children}</main>
        <Footer />
      </body>
    </html>
  );
}
