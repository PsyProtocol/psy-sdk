import axios from "axios";
import SpeculosTransport from "@ledgerhq/hw-transport-node-speculos-http";
import { DisconnectedDevice } from "@ledgerhq/errors";

const connectSpeculos = () => {
  const opts = {
    baseURL: "http://127.0.0.1:8010",
  };
  const axiosInstance = axios.create(opts);
  const speculosTransport = new SpeculosTransport(axiosInstance as any, opts);
  
  //speculosTransport.decorateAppAPIMethods = ()=>0;
  return new Promise<SpeculosTransport>((resolve, reject) => {
    const evtSource = new EventSource(opts.baseURL + "/events?stream=true");
    evtSource.onmessage = (event) => {
      console.log("Event Source data", event);
      speculosTransport.automationEvents.next(JSON.parse(event.data));
      resolve(speculosTransport);
    };
    evtSource.onerror = (event) => {
      console.error("Event Source Error: ", event);
      speculosTransport.emit(
        "disconnect",
        new DisconnectedDevice("Speculos exited!")
      );
      reject(new Error("Speculos exited"));
    };
    /*

    axiosInstance({
      url: "/events?stream=true",
      responseType: "stream",
    })
      .then((response) => {
        response.data.on("data", (d) => {
          console.log('Events Stream data', { d});
        });
        response.data.on("close", () => {
          speculosTransport.emit(
            "disconnect",
            new DisconnectedDevice("Speculos exited!")
          );
        });
        speculosTransport.eventStream = response.data;
        // we are connected to speculos
        resolve(speculosTransport);
      })
      .catch((error) => {
        console.error(JSON.stringify(error));
        reject(error);
      });*/
  });
};

export { connectSpeculos };
