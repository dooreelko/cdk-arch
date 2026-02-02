export const extractContext = <TCast>(that: any) => {
  if (!that) {
    throw new Error('Context is null or underfined');
  }

  if (!that.runtimeContext) {
    throw new Error('Context is missing runtimeContext marker');
  }

  return that as TCast;
};

export type DemoRequest = {
  request: {
    url: string
  }
};