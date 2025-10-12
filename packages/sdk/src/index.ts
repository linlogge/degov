import { createConnectTransport } from "@connectrpc/connect-web";
export { createClient } from "@connectrpc/connect";
export { createConnectTransport } from "@connectrpc/connect-web";

export const defaultTransport = createConnectTransport({
    baseUrl: "/rpc",
});
