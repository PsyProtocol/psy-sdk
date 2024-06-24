
import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import "./fonts/codeicon.css";
import "./styles/flexLayoutTheme.scss";
import {
  RouterProvider,
} from "react-router-dom";
import { router } from './routes';

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
      <RouterProvider router={router} />
  </React.StrictMode>
);

function removeLoader() {
  //showDevConsoleMessage();
  const loader = document.getElementById("loadingOverlay");
  if(loader){
    loader.style.opacity = "0";
    setTimeout(()=>{
      document.body.removeChild(loader);
    }, 500);
  }
  //initiateLoadMonaco();
}
removeLoader();