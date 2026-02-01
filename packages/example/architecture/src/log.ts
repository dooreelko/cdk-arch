export const trace = (what: string, args: any) => {}; //console.log({message: what, level: 'trace', extra: args});
export const log = (what: string, args: any) => console.log({message: what, level: 'info', extra: args});
export const err = (what: string, args: any) => console.log({message: what, level: 'error', extra: args});
