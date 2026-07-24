import { ClientProvider } from "@solana/react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { client } from "./lib/client";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ClientProvider client={client}>
      <App />
    </ClientProvider>
  </StrictMode>,
);
