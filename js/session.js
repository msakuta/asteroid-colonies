
export let sessionId = null;
export let websocket = null;
export let tickTime = 0.2;

export async function fetchSessionId({port, baseUrl, game}) {
    let loaded = false;
    for (let i = 0; i < 20; i++) {
        try {
            const sessionRes = await fetch(`${baseUrl}/api/session`, {
                method: "POST"
            });
            sessionId = await sessionRes.text();
            const dataRes = await fetch(`${baseUrl}/api/load`);
            const dataText = await dataRes.text();
            game.deserialize(dataText);
            tickTime = await (await fetch(`${baseUrl}/api/tick_time`)).json();
            loaded = true;
        } catch (e) {
            console.log(`session api returned an error: ${e}`);
        }
        if (loaded) break;
    }
}

export function reconnectWebSocket({baseUrl, game, onupdate = () => {}}){
    if(sessionId){
        // Is there a smarter way to switch protocol?
        const wsUrl = location.protocol === "https:" ? baseUrl.replace("https", "wss") : baseUrl.replace("http", "ws");
        websocket = new WebSocket(`${wsUrl}/ws/${sessionId}`);
        websocket.binaryType = "arraybuffer";
        websocket.addEventListener("message", (event) => {
            if (event.data instanceof ArrayBuffer) {
                const byteArray = new Uint8Array(event.data);
                game.deserialize_bin(byteArray);
                postChunksDigest(game);
                onupdate();
            }
            else {
            // console.log(`Event through WebSocket: ${event.data}`);
                const data = JSON.parse(event.data);
                if(data.type === "clientUpdate"){
                    if(game){
                        game.deserialize(data.payload);
                        postChunksDigest();
                        onupdate();
                    }
                    // const payload = data.payload;
                    // const body = CelestialBody.celestialBodies.get(payload.bodyState.name);
                    // if(body){
                    //     body.clientUpdate(payload.bodyState);
                    // }
                }
            }
        });
        websocket.addEventListener("open", () => postChunksDigest(game));
    }
}

function postChunksDigest(game) {
    game.uniformify_tiles();
    const chunksDigest = game.serialize_chunks_digest();
    // websocket.send(JSON.stringify({type: "ChunksDigest", payload: {chunks_digest: chunksDigest}}));
    websocket.send(chunksDigest);
}
