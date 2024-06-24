interface IBaseEvent<T> {
  type: T;
}
type IndexableType = string | number | symbol;

type TEventListenerMap<T extends IndexableType, TEvents extends IBaseEvent<T>> = { [K in T]?: ((event: TEvents & {type: K}) => any )[]};


class EventHub<TEventType extends IndexableType, TEvents extends IBaseEvent<TEventType>> {
  eventListeners: TEventListenerMap<TEventType, TEvents> = {};
  addEventListener<T extends TEventType>(
    type: T,
    listener: (event: TEvents & {type: T}) => any
  ): boolean {
    if(typeof listener !== 'function'){
      throw new Error("you cannot add an event listener that is not a function");
    }
    const eventListenerList = this.eventListeners[type];
    if (
      Object.hasOwnProperty.call(this.eventListeners, type) &&
      eventListenerList &&
      Array.isArray(eventListenerList)
    ) {
      if (eventListenerList.indexOf(listener) === -1) {
        eventListenerList.push(listener);
        return true;
      } else {
        return false;
      }
    } else {
      this.eventListeners[type] = [listener];
      return true;
    }
  }
  on<T extends TEventType>(
    type: T,
    listener: (event: TEvents & {type: T}) => any
  ): boolean {
    return this.addEventListener(type, listener);
  }
  onOneOf(
    types: TEventType[],
    listener: (event: TEvents) => any
  ): boolean {
    for(const type of types){
      this.addEventListener(type, listener);
    }
    return true;
  }
  removeOneOf(
    types: TEventType[],
    listener: (event: TEvents) => any
  ): boolean {
    for(const type of types){
      this.removeEventListener(type, listener);
    }
    return true;
  }
  remove<T extends TEventType>(
    type: T,
    listener: (event: TEvents & {type: T}) => any
  ): boolean {
    return this.removeEventListener(type, listener);
  }
  once<T extends TEventType>(
    type: T,
    listener: (event: TEvents & {type: T}) => any
  ): ()=>boolean  {
    let called = false;
    const realListener = (event: any)=>{
      if(called){
        return;
      }
      called = true;
      this.removeEventListener(type, realListener);
      listener(event);
    };
    this.addEventListener(type, realListener);
    return ()=>this.removeEventListener(type, realListener);
  }
  onceFilter<T extends TEventType>(
    type: T,
    filter: (event: TEvents & {type: T}) => boolean,
    listener: (event: TEvents & {type: T}) => any
  ): ()=>boolean  {
    let called = false;
    const realListener = (event: any)=>{
      if(called){
        return;
      }
      if(filter(event)){
        called = true;
        this.removeEventListener(type, realListener);
        listener(event);
      }
    };
    this.addEventListener(type, realListener);
    return ()=>this.removeEventListener(type, realListener);
  }
  removeEventListener<T extends TEventType>(
    type: T,
    listener: (event: TEvents & {type: T}) => any
  ): boolean {
    if (
      Object.hasOwnProperty.call(this.eventListeners, type) &&
      Array.isArray(this.eventListeners[type])
    ) {
      const listeners = this.eventListeners[type];
      if(!listeners){
        return false;
      }
      const index = listeners.indexOf(listener);
      if (index !== -1) {
        if (listeners.length === 0) {
          this.eventListeners[type] = [];
          delete this.eventListeners[type];
        } else {
          this.eventListeners[type] = listeners
            .slice(0, index)
            .concat(listeners.slice(index + 1));
        }
        return true;
      } else {
        return false;
      }
    } else {
      return false;
    }
  }
  removeAllEventListeners<T extends TEventType>(type: T) {
    this.eventListeners[type] = [];
    delete this.eventListeners[type];
  }
  private notifyWithErrorsInternal<T extends TEventType>(event: TEvents & {type: T}): any[]{
    const errors : any[] = [];

    if (
      Object.hasOwnProperty.call(this.eventListeners, event.type) &&
      Array.isArray(this.eventListeners[event.type]) &&
      (this.eventListeners[event.type] as any).length
    ) {
      const listeners = this.eventListeners[event.type] as any;
      for(const listener of listeners){
        try {
          listener(event);
        }catch(err: any){
          errors.push(err);
        }
      }
    }
    return errors;
  }
  

  notifyWithErrors<T extends TEventType>(type: T, event: Omit<TEvents & {type: T}, "type">): any[];
  notifyWithErrors<T extends TEventType>(event: TEvents & {type: T}): any[];
  notifyWithErrors<T extends TEventType>(eventOrType: T | TEvents & {type: T}, event?: Omit<TEvents & {type: T}, "type">): any[]{
    if(typeof event === 'object' && typeof eventOrType !== 'object'){
      return this.notifyWithErrorsInternal({type: eventOrType, ...event} as TEvents & {type: T});
    }else{
      return this.notifyWithErrorsInternal(eventOrType as TEvents & {type: T});
    }
  }

  notify<T extends TEventType>(type: T, event: Omit<TEvents & {type: T}, "type">): void;
  notify<T extends TEventType>(event: TEvents & {type: T}): void;
  notify<T extends TEventType>(eventOrType: T | TEvents & {type: T}, event?: Omit<TEvents & {type: T}, "type">): void{
    if(typeof event === 'object' && typeof eventOrType !== 'object'){
      this.notifyInternal({type: eventOrType, ...event} as TEvents & {type: T});
    }else{
      this.notifyInternal(eventOrType as TEvents & {type: T});
    }
  }
  private notifyInternal<T extends TEventType>(event: TEvents & {type: T}): void{
    console.log("notifyInternal", event);
    if (
      Object.hasOwnProperty.call(this.eventListeners, event.type) &&
      Array.isArray(this.eventListeners[event.type]) &&
      (this.eventListeners[event.type] as any).length
    ) {
      const listeners = (this.eventListeners[event.type] as any);
      for(const listener of listeners){
        try {
          listener(event);
        }catch(err: any){
          // do nothing
        }
      }
    }
  }
}

export type {
  IBaseEvent,
  IndexableType,
  TEventListenerMap,
}
export {
  EventHub,
}