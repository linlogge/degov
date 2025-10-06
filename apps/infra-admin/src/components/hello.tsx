import { type HelloRequest, HelloWorldService } from "@degov/sdk/pb/hello_pb";
import { createClient, transport } from "@degov/sdk";
import { useState } from "react";

const client = createClient(HelloWorldService, transport);

export const Hello = () => {
    const [name, setName] = useState("");
    const [message, setMessage] = useState("");

    const handleSubmit = async () => {
        const response = await client.sayHello({ name });
        setMessage(response.message);
    };

    return <div>
        <input type="text" value={name} onChange={(e) => setName(e.target.value)} />
        <button onClick={handleSubmit}>Submit</button>
        <p>{message}</p>
    </div>;
};
