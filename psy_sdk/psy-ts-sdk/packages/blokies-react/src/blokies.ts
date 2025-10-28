type BlokieRNG = () => number;
interface IBlokiesInputOpts {
    seed: Uint8Array | string;
    size?: number;
    scale?: number;
    color?: string;
    bgColor?: string;
    spotColor?: string;
}
interface IBlokiesFinalOptsBase {
    size: number;
    scale: number;
    color: string;
    bgColor: string;
    spotColor: string;
}
interface IBlokiesFinalOpts extends IBlokiesFinalOptsBase {
    rand: BlokieRNG;
}

const DEFAULT_OPTS: IBlokiesFinalOptsBase = {
    size: 8,
    scale: 4,
    color: "",
    bgColor: "",
    spotColor: "",
};
const randSeed = new Array(4); // Xorshift: [x, y, z, w] 32 bit values

function generateRandSeedU8Array(seed: Uint8Array) {
    const randSeed = new Uint32Array(4);
    for (let i = 0; i < seed.length; i++) {
        randSeed[i & 3] = (randSeed[i & 3] << 5) - randSeed[i & 3] + seed[i];
    }
    return randSeed;
}

function rand(randSeed: Uint32Array): number {
    // based on Java's String.hashCode(), expanded to 4 32bit values
    const t = randSeed[0] ^ (randSeed[0] << 11);

    randSeed[0] = randSeed[1];
    randSeed[1] = randSeed[2];
    randSeed[2] = randSeed[3];
    randSeed[3] = randSeed[3] ^ (randSeed[3] >> 19) ^ t ^ (t >> 8);

    return (randSeed[3] >>> 0) / ((1 << 31) >>> 0);
}
function randHelper(baseSeed: Uint8Array) {
    const randSeed = generateRandSeedU8Array(baseSeed);
    return () => rand(randSeed);
}

function createColor(rng: BlokieRNG) {
    //saturation is the whole color spectrum
    const h = Math.floor(rng() * 360);
    //saturation goes from 40 to 100, it avoids greyish colors
    const s = rng() * 60 + 40 + "%";
    //lightness can be anything from 0 to 100, but probabilities are a bell curve around 50%
    const l = (rng() + rng() + rng() + rng()) * 25 + "%";

    return "hsl(" + h + "," + s + "," + l + ")";
}

function createImageData(size: number, rng: BlokieRNG) {
    const width = size; // Only support square icons for now
    const height = size;

    const dataWidth = Math.ceil(width / 2);
    const mirrorWidth = width - dataWidth;

    const data = [];
    for (let y = 0; y < height; y++) {
        let row = [];
        for (let x = 0; x < dataWidth; x++) {
            // this makes foreground and background color to have a 43% (1/2.3) probability
            // spot color has 13% chance
            row[x] = Math.floor(rng() * 2.3);
        }
        const r = row.slice(0, mirrorWidth);
        r.reverse();
        row = row.concat(r);

        for (let i = 0; i < row.length; i++) {
            data.push(row[i]);
        }
    }
    return data;
}

function buildOpts(opts: IBlokiesInputOpts): IBlokiesFinalOpts {
    const seed = typeof opts.seed === "string" ? new TextEncoder().encode(opts.seed) : opts.seed;
    const rand = randHelper(seed);

    return {
        rand,
        size: opts.size || 8,
        scale: opts.scale || 4,
        color: opts.color || createColor(rand),
        bgColor: opts.bgColor || createColor(rand),
        spotColor: opts.spotColor || createColor(rand),
    };
}

function renderBlokiesIcon(inputOpts: IBlokiesInputOpts, canvas: HTMLCanvasElement) {
    const opts = buildOpts(inputOpts);
    const imageData = createImageData(opts.size, opts.rand);
    const width = Math.sqrt(imageData.length);

    canvas.width = canvas.height = opts.size * opts.scale;

    const cc = canvas.getContext("2d");
    if (!cc) throw new Error("Canvas context not found");
    cc.fillStyle = opts.bgColor;
    cc.fillRect(0, 0, canvas.width, canvas.height);
    cc.fillStyle = opts.color;

    for (let i = 0; i < imageData.length; i++) {
        // if data is 0, leave the background
        if (imageData[i]) {
            const row = Math.floor(i / width);
            const col = i % width;

            // if data is 2, choose spot color, if 1 choose foreground
            cc.fillStyle = imageData[i] == 1 ? opts.color : opts.spotColor;
            cc.fillRect(col * opts.scale, row * opts.scale, opts.scale, opts.scale);
        }
    }

    return canvas;
}

export type { IBlokiesInputOpts };
export { renderBlokiesIcon };
