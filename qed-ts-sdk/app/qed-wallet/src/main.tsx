import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

import "@mantine/core/styles.css";
import "@qed/qed-wallet-widget/src/themes/variables.css";
import { RouterProvider } from "react-router-dom";
import { MantineProvider } from "@mantine/core";
import { router } from "./routes";

// Import extension styles if running as Chrome extension
if (window.location.protocol === 'chrome-extension:') {
    import('./extension.css');
    import('./extension-overrides.css');
}
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <MantineProvider defaultColorScheme="light">
            <RouterProvider router={router} />
        </MantineProvider>
    </React.StrictMode>
);
