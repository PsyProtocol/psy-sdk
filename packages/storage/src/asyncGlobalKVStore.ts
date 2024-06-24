import { IAsyncGlobalKVStore } from "./types";

class AsyncGlobalKVStore implements IAsyncGlobalKVStore{

  keyPrefix: string;
  store: LocalForage;
  constructor(keyPrefix: string, store: LocalForage) {
    this.keyPrefix = keyPrefix;
    this.store = store;
  }
  getFullKey(key: string): string {
    return this.keyPrefix + key;
  }
  async getItem<T>(key: string): Promise<T | null> {
    return this.store.getItem<T>(this.getFullKey(key));
  }
  async setItem<T>(key: string, value: T): Promise<void> {
    await this.store.setItem<T>(this.getFullKey(key), value);
  }
  async removeItem(key: string): Promise<void> {
    return this.store.removeItem(this.getFullKey(key));
  }
  async addToSet<T>(key: string, item: T, compare: (a: T, b: T) => boolean, replace?: boolean | undefined): Promise<T[]> {
   const items = await this.getItem<T[]>(key);
    if(items === null){
      await this.setItem<T[]>(key,[item]);
      return [item];
    }else if(Array.isArray(items)){
      for(let i=0; i<items.length; i++){
        if(compare(items[i], item)){
          if(replace){
            items[i] = item;
            await this.setItem<T[]>(key, items);
            return items;
          }else{
            return items;
          }
        }
      }
      const newItems = items.concat([item]);
      await this.setItem<T[]>(key, newItems);
      return newItems;
    }else{
      throw new Error("Cannot add item to set for key "+key+", value is not an array");
    }
  }
  async removeFromSet<T>(key: string, item: T, compare: (a: T, b: T) => boolean): Promise<T[]> {
    const items = await this.getItem<T[]>(key);
    if(items === null){
      return [];
    }else if(Array.isArray(items)){
      const newItems = items.filter((i) => !compare(i, item));
      await this.setItem<T[]>(key, newItems);
      return newItems;
    }else{
      throw new Error("Cannot remove item from set for key "+key+", value is not an array");
    }
  }

}

export {
  AsyncGlobalKVStore,
}