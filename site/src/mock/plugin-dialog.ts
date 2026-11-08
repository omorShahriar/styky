/** The demo has no file system: dialogs resolve as "cancelled". */
export const ask = async (..._args: unknown[]): Promise<boolean> => false;
export const open = async (..._args: unknown[]): Promise<string | string[] | null> => null;
export const save = async (..._args: unknown[]): Promise<string | null> => null;
