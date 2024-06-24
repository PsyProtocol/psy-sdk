import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

import '@mantine/core/styles.css';
import * as dogeJs from "@qstudio/doge-js";

import {
  RouterProvider,
} from "react-router-dom";
import { MantineProvider } from '@mantine/core';
(window as any).dg = dogeJs;
import { router } from './routes';
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MantineProvider defaultColorScheme="dark">
      <RouterProvider router={router} />
    </MantineProvider>
  </React.StrictMode>
);