import { HelloWorldService } from "@dgv/sdk/pb/hello_pb";
import { useQuery } from "@dgv/sdk/query";
import { useState } from "react";

export const Hello = () => {
    const [name, setName] = useState("");
    const { data, refetch } = useQuery(HelloWorldService.method.sayHello, {
        name: name,
    });

    return <div>
        <input type="text" value={name} onChange={(e) => setName(e.target.value)} />
        <button onClick={() => refetch()}>Submit</button>
        <p>{data?.message}</p>
    </div>;
};
