import { test, expect } from "bun:test";
import axios from "axios";
import { password } from "bun";

const BACKEND_URL: string = "http://localhost:8080";

const api = axios.create({
    baseURL: BACKEND_URL,
    validateStatus: () => true
});

test("New user gets created Succressfully!", async () => {
    const response = await api.post("/signup", {
        username: `arpit ${Date.now()}`,
        password: "123"
    });

    expect(response.status).toBe(200);
    expect(response.data.message).toBe("Signup successfull!");
})

test("Adding user balance!", async () => {
    const response = await api.post("/signup", {
        username: `test_username ${Date.now()}`,
        password: "test_password"
    });

    expect(response.status).toBe(200);

    let token = response.data.token;

    let onrampResponse = await api.post("/protected/onramp",
        { qty: 300 },
        { headers: { Authorization: `Bearer ${token}` } });

    expect(onrampResponse.status).toBe(200);
    expect(onrampResponse.data.msg).toBe("Balance updated!");
})

test("Adding bid", async () => {

    let username = `bid ${Date.now()}`;
    let password = "qwasqwas";

    let resposne = await api.post("/signup", {
        username,
        password
    });

    await api.post("/protected/onramp",
        { qty: 500 },
        { headers: { Authorization: `Bearer ${resposne.data.token}` } }
    );

    let makingBid = await api.post("/protected/orders",
        {
            header: "bid",
            order_type: "market",
            price: 10,
            qty: 2,
            asset: "sol"
        },
        {
            headers:
                { Authorization: `Bearer ${resposne.data.token}` }
        });

    await api.post("/protected/orders",
        {
            header: "bid",
            order_type: "market",
            price: 20,
            qty: 2,
            asset: "sol"
        },
        {
            headers: {
                Authorization: `Bearer ${resposne.data.token}`
            }
        }
    );
    await api.post("/protected/orders",
        {
            header: "bid",
            order_type: "market",
            price: 20,
            qty: 3,
            asset: "sol"
        },
        {
            headers: {
                Authorization: `Bearer ${resposne.data.token}`
            }
        }
    );

    expect(makingBid.status).toBe(200);
    expect(makingBid.data.msg).toBe("Order placed successfully!");
})

test("Making ask", async () => {

    let response = await api.post("/signup", {
        username: `ask ${Date.now()}`,
        password: "qwasqwas"
    });

    await api.post("/protected/onramp",
        { qty: 500 },
        {
            headers:
            {
                Authorization: response.data.token
            }
        }
    );

    let ask_response = await api.post("/protected/orders",
        {
            header: "ask",
            order_type: "market",
            price: 30,
            qty: 2,
            asset: "sol"
        },
        {
            headers: {
                Authorization: response.data.token
            }
        }
    );

    await api.post("/protected/orders",
        {
            header: "ask",
            order_type: "market",
            price: 20,
            qty: 4,
            asset: "sol"
        },
        {
            headers: {
                Authorization: response.data.token
            }
        }
    )

    expect(ask_response.status).toBe(200);
    expect(ask_response.data.msg).toBe("Order placed successfully!");

})