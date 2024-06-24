import { QEDVizPaper } from "@qstudio/core";
import { useEffect, useRef } from "react";

interface IQVizRendererProps {
  width?: number;
  height?: number;
  onRendererManager?: (manager?: QEDVizPaper)=>void;
}
const QVizRenderer = ({width, height, onRendererManager}: IQVizRendererProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const qvPaper = useRef<QEDVizPaper>();
  useEffect(()=>{
    let qvp = qvPaper.current;
    if(containerRef.current){
      let bcr = containerRef.current.getBoundingClientRect();

      qvp = new QEDVizPaper(containerRef.current, {width: width || bcr.width, height: height || bcr.height});
      qvPaper.current = qvp;
    }
    if(qvp){
      qvp.resizeToFit();
    }
    return ()=>{
      if(qvp){
        qvp.dispose();
      }
    }

  },[width,height, containerRef, qvPaper]);
  useEffect(()=>{
    if(onRendererManager){
      onRendererManager(qvPaper.current);
    }
  },[onRendererManager, qvPaper]);
  useEffect(()=>{
    if(qvPaper.current){
      qvPaper.current.resizeToFit();
    }
  },[qvPaper, containerRef, onRendererManager])
  const conStyle = {
    display: "inline-block",
    border: "1px solid #000",
    backgroundColor: "#181919",
    //backgroundImage: "radial-gradient(#444cf7 0.5px, #feffe0 0.5px)",
    backgroundSize: "10px 10px",
    width:"100%",
    height:"100%",
  };
  return(
    <div className="qed-viz-renderer" ref={containerRef} style={conStyle}></div>
  )
}

export default QVizRenderer;