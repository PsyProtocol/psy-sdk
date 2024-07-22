
import AboutPage from "./about";
import HomePage from "./home";
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
  }
]);


export {
  router,
}