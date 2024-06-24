
import AboutPage from "./about";
import HomePage from "./home";
import LedgerPage from "./ledgerTest";
import {
  createBrowserRouter,
} from "react-router-dom";

const router: any = createBrowserRouter([
  {
    path: "/",
    element: <HomePage />,
  },
  {
    path: "/about",
    element: <AboutPage />,
  },
  {
    path: "/ledger-test",
    element: <LedgerPage />,
  }
]);


export {
  router,
}