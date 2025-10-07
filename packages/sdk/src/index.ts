import { createConnectTransport } from "@connectrpc/connect-web";
export { createClient } from "@connectrpc/connect";

export const transport = createConnectTransport({
    baseUrl: "/rpc",
});
