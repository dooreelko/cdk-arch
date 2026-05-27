import { getCurrentContext } from '@arinoto/cdk-arch';

export const extractContext = <TCast>() => {
  const ctx = getCurrentContext();
  if (!ctx) {
    throw new Error('Context is null or undefined');
  }

  return ctx as TCast;
};

export type DemoRequest = {
  request: {
    url: string
  }
};