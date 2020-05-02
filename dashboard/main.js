const ws = new WebSocket("ws://localhost:9000")

const canvas = document.querySelector("canvas")

const ctx = canvas.getContext("2d")
ctx.imageSmoothingEnabled = false

const tileColours = [
    // Nothing
    "white",
    // Block
    "black",
    // Enemy
    "orange",
    // Mario
    "red"
]

const renderScreen = (screen) => {
	screen.forEach((row, rowIndex) => {
		row.forEach((tile, tileIndex) => {
			ctx.fillStyle = tileColours[tile]
			ctx.fillRect(tileIndex, rowIndex, 1, 1)
		})
	})
}

let renderScreenTimeout = null

ws.addEventListener("message", (e) => {
	const { event, data } = JSON.parse(e.data)
	switch (event) {
	case "update_screen":
        if (renderScreenTimeout) {
            cancelAnimationFrame(renderScreenTimeout)
        }
        renderScreenTimeout = requestAnimationFrame(() => renderScreen(data))
    }
})
