
import AboutPage from "./about";
import {
  createBrowserRouter,
} from "react-router-dom";
import StudioPage from "./studio";

const router: any = createBrowserRouter([
  {
    path: "/",
    element: <StudioPage />,
  },
  {
    path: "/about",
    element: <AboutPage />,
  }
]);


export {
  router,
}