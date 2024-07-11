import { hexToU8Array, u8ArrayToHex } from "doge-sdk/dist/types";
import { HostMessageTypes, WorkerMessageTypes } from "./types";

function randomId(){
  return [
    Math.floor(0x100000000*Math.random()).toString(36),
    Math.floor(0x100000000*Math.random()).toString(36),
    Math.floor(0x100000000*Math.random()).toString(36),
    Math.floor(0x100000000*Math.random()).toString(36),
    Math.floor(0x100000000*Math.random()).toString(36),
    Math.floor(0x100000000*Math.random()).toString(36),
    Math.floor(0x100000000*Math.random()).toString(36),
    Math.floor(0x100000000*Math.random()).toString(36),
  ].join("-");
}


interface IHandler {
  resolve: (value: any) => void;
  reject: (reason?: any) => void;
}
class ScryptWorkerManager {
  scriptPath: string;
  worker?: Worker;
  handlerMap: Record<string, IHandler> = {};
  constructor(scriptPath: string) {
    this.scriptPath = scriptPath;
    this.onMessage = this.onMessage.bind(this);
  }
  onMessage(e: MessageEvent){
    if(typeof e.data === 'object' && e.data && e.data.id && e.data.type){
      const handler = this.handlerMap[e.data.id];
      if(handler){
        if(e.data.type === WorkerMessageTypes.ScryptResult){
          handler.resolve(hexToU8Array(e.data.data));
          delete this.handlerMap[e.data.id];
        }else if(e.data.type === WorkerMessageTypes.ScryptError){
          handler.reject(new Error(e.data.data));
          delete this.handlerMap[e.data.id];
        }
      }
    }
  }
  ensureWorker(): Promise<Worker> {
    if(this.worker){
      return Promise.resolve(this.worker);
    }
    return new Promise((resolve, reject)=>{
    const worker = new Worker(this.scriptPath);
    worker.onerror = (e)=>{
      reject(e);
    };
    worker.onmessage = ()=>{
      worker.onmessage = this.onMessage;
      this.worker = worker;
      resolve(worker);
    };
  });
  }
  async scrypt(
    password: Uint8Array,
    salt: Uint8Array,
    N: number,
    r: number,
    p: number,
    dkLen: number
  ): Promise<Uint8Array> {
    const worker = await this.ensureWorker();
    const id = randomId();
    const payload = {
      password: u8ArrayToHex(password),
      salt: u8ArrayToHex(salt),
      N,
      r,
      p,
      dkLen,
    };
    worker.postMessage({
      id,
      type: HostMessageTypes.Scrypt,
      data: payload,
    });
    return new Promise<Uint8Array>((resolve, reject) => {
      this.handlerMap[id] = { resolve, reject };
    });
  }
}

const workerManager = new ScryptWorkerManager("/workers/scrypt.js");
function scrypt(
  password: Uint8Array,
  salt: Uint8Array,
  N: number,
  r: number,
  p: number,
  dkLen: number
): Promise<Uint8Array> {
  return workerManager.scrypt(password, salt, N, r, p, dkLen);
}

export{
  scrypt,
}