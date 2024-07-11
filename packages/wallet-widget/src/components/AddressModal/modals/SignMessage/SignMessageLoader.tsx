import React from "react";
import { Loader, LoadingOverlay, LoadingOverlayProps } from "@mantine/core";


const SignMessageLoaderBody = () => {
  return (
    <div style={{display:"flex",alignItems:"center", justifyContent:"center",flexDirection:"column"}}>
      <Loader size="md" />
      <div style={{paddingTop: "12px"}}>Please confirm the signature on your device</div>
    </div>
  );
}
const SignMessageLoader: React.FC<LoadingOverlayProps> = (props) => {
  return <LoadingOverlay {...props} loaderProps={{ children: <SignMessageLoaderBody /> }} />;
};

export {SignMessageLoader};