import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import "./styles/flexLayoutTheme.scss";
import "./styles/cmdk/cmdk.scss";
import "./styles/proof.scss";
import "./styles/contextify/main.scss";
import '@mantine/core/styles.css';

import {
  RouterProvider,
} from "react-router-dom";
import { MantineProvider } from '@mantine/core';

import { router } from './routes';
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MantineProvider defaultColorScheme="dark">
      <RouterProvider router={router} />
    </MantineProvider>
  </React.StrictMode>
);