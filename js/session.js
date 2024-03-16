export let websocket = null;

export async function getSessionId(setSessionId) {
    let loaded = false;
    for (let i = 0; i < 20; i++) {
        try {
            const sessionRes = await fetch(`http://${location.hostname}:${port}/api/session`, {
                method: "POST"
            });
            setSessionId(await sessionRes.text());
            const dataRes = await fetch(`${baseUrl}/api/load`);
            const dataText = await dataRes.text();
            game.deserialize(dataText);
            loaded = true;
        } catch (e) {
            console.log(`session api returned an error: ${e}`);
        }
        if (loaded) break;
    }
}

export function reconnectWebSocket(){
    if(sessionId){
        websocket = new WebSocket(`ws://${location.hostname}:${port}/ws/${sessionId}`);
        websocket.binaryType = "arraybuffer";
        websocket.addEventListener("message", (event) => {
            if (event.data instanceof ArrayBuffer) {
                const byteArray = new Uint8Array(event.data);
                game.deserialize_bin(byteArray);
                postChunksDigest();
                heartbeatOpacity = 1;
                updateHeartbeatOpacity();
            }
            else {
            // console.log(`Event through WebSocket: ${event.data}`);
                const data = JSON.parse(event.data);
                if(data.type === "clientUpdate"){
                    if(game){
                        game.deserialize(data.payload);
                        postChunksDigest();
                        heartbeatOpacity = 1;
                        updateHeartbeatOpacity();
                    }
                    // const payload = data.payload;
                    // const body = CelestialBody.celestialBodies.get(payload.bodyState.name);
                    // if(body){
                    //     body.clientUpdate(payload.bodyState);
                    // }
                }
            }
        });
        websocket.addEventListener("open", postChunksDigest);
    }
}
