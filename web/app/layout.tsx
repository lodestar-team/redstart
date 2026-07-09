import type { Metadata } from "next";
import { Space_Grotesk } from "next/font/google";
import { GeistSans } from "geist/font/sans";
import { GeistMono } from "geist/font/mono";
import "./globals.css";
import { Nav } from "@/components/nav";
import { Footer } from "@/components/footer";
import { Cosmos } from "@/components/cosmos";
import { VersionWatcher } from "@/components/version-watcher";

const display = Space_Grotesk({
  variable: "--font-display",
  subsets: ["latin"],
  weight: ["500", "600", "700"],
});

const SITE = "https://redstart-lang.com";
const TAGLINE = "Redstart — the best language for building The Graph subgraphs";
const DESCRIPTION =
  "The most performant and secure language for authoring The Graph subgraphs. One typed source for schema, manifest, and mappings — compiled to AssemblyScript that's faster and safer than any human would hand-write. If it compiles, it works.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE),
  title: {
    default: TAGLINE,
    template: "%s · Redstart",
  },
  description: DESCRIPTION,
  openGraph: {
    title: TAGLINE,
    description: DESCRIPTION,
    url: SITE,
    siteName: "Redstart",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: TAGLINE,
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
        <VersionWatcher />
      </body>
    </html>
  );
}
