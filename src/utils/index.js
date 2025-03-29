export function getImageUrl(name, ext) {
    return new URL(`../assets/${name}`, import.meta.url).href
}