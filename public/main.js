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

const updateScreen = (screen) => {
	screen.forEach((row, rowIndex) => {
		row.forEach((tile, tileIndex) => {
			ctx.fillStyle = tileColours[tile]
			ctx.fillRect(tileIndex, rowIndex, 1, 1)
		})
	})
}

ws.addEventListener("message", (event) => {
	const { event, data } = JSON.parse(event.data)
	switch (event) {
	case "update_screen":
		updateScreen(data)
    }
})
