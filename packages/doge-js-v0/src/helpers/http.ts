import { IDogeHTTPClient, ISimpleHTTPRequest, ISimpleHTTPResponse } from "../types/http";

class FetchHTTPClient implements IDogeHTTPClient {
  async sendRequest(request: ISimpleHTTPRequest): Promise<ISimpleHTTPResponse> {
    //@ts-ignore
    const result = await fetch(request.url, {
      method: request.method,
      headers: request.headers,
      body: request.body,
      credentials: request.credentials,
    });
    if(!result.ok){
      if(request.responseType === "json"){
        try {
          const body = await result.json();
          return {
            statusCode: result.status,
            body,
          };
        }catch(e){
          return {
            statusCode: result.status,
            body: null,
          };
        }
      }
    }
    if(request.responseType === "json"){
      return {
        statusCode: result.status,
        body: await result.json(),
      };
    }else if(request.responseType === "text"){
      return {
        statusCode: result.status,
        body: await result.text(),
      };
    }else{
      return {
        statusCode: result.status,
        body: await result.arrayBuffer(),
      };
    }

  }
}

const fetchHTTPClient = new FetchHTTPClient();

export {
  FetchHTTPClient,
  fetchHTTPClient,
}