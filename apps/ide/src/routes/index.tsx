
import AboutPage from "./about";
import IDEPage from "./ide";
import {
  createBrowserRouter,
} from "react-router-dom";

const router: any = createBrowserRouter([
  {
    path: "/",
    element: <IDEPage />,
  },
  {
    path: "/about",
    element: <AboutPage />,
  }
]);


export {
  router,
}