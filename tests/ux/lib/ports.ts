const port = (name: string, fallback: number) => Number(process.env[name] ?? fallback);

export const API_PORT = port('ATLAS_API_PORT', 8000);
export const CLIENT_PORT = port('ATLAS_CLIENT_PORT', 5000);
export const API = `http://localhost:${API_PORT}`;
export const CLIENT = `http://localhost:${CLIENT_PORT}`;
export const portEnv = { ATLAS_API_PORT: String(API_PORT), ATLAS_CLIENT_PORT: String(CLIENT_PORT) };
