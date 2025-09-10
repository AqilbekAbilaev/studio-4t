import { h } from "vue"

const vnode = h(
    'div', // type
    { id: 'foo', class: 'bar', innerHTML: "Hi world!" }, // props
    [
        /* children */
    ]
)

