import "./style.css";

export const metadata = {
  title: "Next Benchmark"
};

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
