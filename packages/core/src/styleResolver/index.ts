type TQVStyleResolver<T, C> = (config: C) => T;

interface IQVizStyleResolver {
  getStyleDef<T, C>(widgetId: string, config: C): T;
}

class QVizStyleResolver implements IQVizStyleResolver {
  resolvers: Record<string, TQVStyleResolver<any, any>> = {};
  registerStyleResolver<T, C>(widgetId: string, config: TQVStyleResolver<T, C>) {
    this.resolvers[widgetId] = config;
  }
  getStyleDef<T, C>(widgetId: string, config: C): T {
    if(Object.hasOwnProperty.call(this.resolvers, widgetId)) {
      const resolver = this.resolvers[widgetId];
      if(typeof resolver === 'function') {
        return resolver(config);
      }
    }
    throw new Error(`QVizStyleResolver: No style resolver found for widgetId: ${widgetId}`);
  }
}
const globalStyleResolver = new QVizStyleResolver();
function registerGlobalQVizStyleResolver<T, C>(widgetId: string, config: TQVStyleResolver<T, C>) {
  globalStyleResolver.registerStyleResolver(widgetId, config);
}

function getGlobalQVizStyleDef<T, C>(widgetId: string, config: C): T {
  return globalStyleResolver.getStyleDef(widgetId, config);
}
export {
  QVizStyleResolver,
  globalStyleResolver,
  registerGlobalQVizStyleResolver,
  getGlobalQVizStyleDef,
}

export type {
  IQVizStyleResolver,
  TQVStyleResolver,
}