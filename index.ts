import { promises as fs } from 'fs'
import path from 'path'

const ROOT = `.`
let FILE_CURSOR_POSITION = 0;

type SaveGame = {
    gameTitle: string
    headerSize: number
    saveNumber: number
    header: string
    version: number
    playerName: string
    playerLevel: number 
    playerLocation: string
}
type Size = 1 | 4
type SizeType = {
    size: Size
}
const u32: SizeType = {
    size: 4
}

function readBuffer(buffer: Buffer, size: number): Buffer {
    let chunk = buffer.slice(FILE_CURSOR_POSITION, FILE_CURSOR_POSITION + size)
    FILE_CURSOR_POSITION = FILE_CURSOR_POSITION + size
    console.log(`cursor moved to position: ${FILE_CURSOR_POSITION}`)
    return chunk
}

function readWstringFromBuffer(buffer: Buffer) {
    let stringSizeBuf = buffer.slice(FILE_CURSOR_POSITION, FILE_CURSOR_POSITION + 2)
    const strLen = stringSizeBuf.readInt16LE()
    FILE_CURSOR_POSITION += 2
    return readBuffer(buffer, strLen).toString()
}

async function processFile(filePath: string) {

    const saveGame: SaveGame = {
        gameTitle: '',
        headerSize: 0,
        header: '',
        saveNumber: 0,
        version: 0,
        playerName: '',
        playerLevel: 0,
        playerLocation: ''
    }
    console.log(`Loading file: ${filePath}`)
    const fileBuffer = await fs.readFile(filePath);

    let token_buffer = []
    
    saveGame.gameTitle = readBuffer(fileBuffer, 13).toString()
    saveGame.headerSize = readBuffer(fileBuffer, 4).readInt32LE()
    saveGame.version = readBuffer(fileBuffer, 4).readInt32LE()
    saveGame.saveNumber = readBuffer(fileBuffer, 4).readInt32LE()
    saveGame.playerName = readWstringFromBuffer(fileBuffer)
    saveGame.playerLevel = readBuffer(fileBuffer, 4).readInt32LE()
    saveGame.playerLocation = readWstringFromBuffer(fileBuffer)

    console.log(`==================================`)
    console.dir(saveGame)
}



async function main() {
    const testSave = 'test.ess'
    console.log(`Testing with save ${testSave}`)

    const filePath = path.join(ROOT, testSave)
    processFile(filePath)
}
main()