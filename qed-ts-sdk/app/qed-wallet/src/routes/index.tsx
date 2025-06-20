import AboutPage from "./about";
import HomePage from "./home";
import ExtensionHomePage from "./home/ExtensionHome";
import Settings from "../components/Settings";
import { createBrowserRouter, createHashRouter } from "react-router-dom";

const isExtension = window.location.protocol === 'chrome-extension:';

// Use HashRouter for extension to avoid path issues
const router: any = isExtension 
    ? createHashRouter([
        {
            path: "/",
            element: <ExtensionHomePage />,
        },
        {
            path: "/settings",
            element: <Settings />,
        },
    ])
    : createBrowserRouter([
        {
            path: "/",
            element: <HomePage />,
        },
        {
            path: "/settings",
            element: <Settings />,
        },
    ]);

export { router };
