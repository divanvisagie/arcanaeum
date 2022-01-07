import { promises as fs } from 'fs'
import os from 'os'
import path from 'path'
import { head, slice } from 'ramda'
import { buffer } from 'stream/consumers'
import util from 'util'

const ROOT = `.`
let FILE_CURSOR_POSITION = 0;

type SaveGame = {
    gameTitle: string
    headerSize: number
    saveNumber: number
    header: string
    version: number
}
type Size = 1 | 8
type SizeType = {
    size: Size
}
const u32: SizeType = {
    size: 8
}

function readBuffer(buffer: Buffer, size: number): Buffer {
    let chunk = buffer.slice(FILE_CURSOR_POSITION, FILE_CURSOR_POSITION + size)
    FILE_CURSOR_POSITION = FILE_CURSOR_POSITION + size
    console.log(`cursor moved to position: ${FILE_CURSOR_POSITION}`)
    return chunk
}

function readNumberWithSize(buffer: Buffer, size: Size): number {
    const slice = readBuffer(buffer, size)
    return slice.readInt32BE()
}

function readChars(buffer: Buffer, count: number): string {
    const slice = readBuffer(buffer, count)
    return slice.toString()
}

async function processFile(filePath: string) {

    const saveGame: SaveGame = {
        gameTitle: '',
        headerSize: 0,
        header: '',
        saveNumber: 0,
        version: 0
    }
    console.log(`Loading file: ${filePath}`)
    const fileBuffer = await fs.readFile(filePath);

    let token_buffer = []
    
    saveGame.gameTitle = fileBuffer.slice(0,13).toString()
    saveGame.headerSize = fileBuffer.slice(13,17).readInt32LE()
    saveGame.version = fileBuffer.slice(17,21).readInt32LE()
    saveGame.saveNumber = fileBuffer.slice(21,25).readInt32LE()

    fileBuffer.slice()

    // let gameTitle = readChars(buf, 13)
    // saveGame.gameTitle = gameTitle

    // let headerSizeBuf = readBuffer(buf, u32.size)
    // saveGame.headerSize = headerSizeBuf.readUInt32LE()

    // // We are now reading from the header

    // let nameWidth = readBuffer(buf, u32.size)
    // saveGame.saveNumber = nameWidth.readUInt32LE()


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