let html = require('choo/html')
let devtools = require('choo-devtools')
let choo = require('choo')
let logo = require('./logo.png')

let app = choo()
app.use(devtools())
app.use(countStore)
app.route('/', mainView)
app.mount('body')

function mainView (state, emit) {
  return html`
    <body>
      <img class="logo" src=${logo} />
      ${chainView()}
      <section class="info"></section>
    </body>
  `
}

function chainView (state, emit) {
  return html`
    <section class="chains">
      <div class="chain">
        <label><span>Bitcoin Testnet</span></label>
        <div class="fade"><div class="link"></div></div>
        ${block()}
        <div class="link"></div>
        ${block()}
        <div class="link"></div>
        ${block()}
        <div class="link mining"></div>
        ${block({ mining: 'Mining' })}
      </div>
      <div class="interchain">
        <div class="link"></div>
        <div class="block"></div>
        <div class="link"></div>
        <div class="block"></div>
      </div>
      <div class="chain secondary">
        <div class="fade"><div class="link"></div></div>
        ${block()}
        <div class="link"></div>
        ${block()}
        <div class="link"></div>
        ${block()}
        <div class="link mining"></div>
        ${block({ mining: 'Validating' })}
        <label><span>Nomic Sidechain Testnet</span></label>
      </div>
    </section>
  `
}

function block (opts = {}) {
  let mining = opts.mining

  if (!mining) {
    return html`
      <div class="block">
        <div class="content">
          <span>#123,456</span>
          <br>
          <span class="hash">00..a1b2c3</span>
          <br>
          <br>
          <!-- <span>
            <label>Foo</label>
            <span>Bar</span><span class="separator"></span><label>Foo</label>
            <span>Bar</span>
          </span> -->
        </div>
      </div>
    `
  } else {
    let r = 8
    let w = 90 - r - 3 * 2
    let h = 70 - r - 3 * 2

    let path = `
      m${r+2},3
      h${w}
      a${r},${r} 0 0 1 ${r},${r}
      v${h}
      a${r},${r} 0 0 1 -${r},${r}
      h-${w}
      a${r},${r} 0 0 1 -${r},-${r}
      v-${h}
      a${r},${r} 0 0 1 ${r},-${r}
    `

    return html`
      <div class="block mining">
        <svg width="100%" height="100%">
          <path
            d=${path}
            stroke="#bbb"
            stroke-width="3"
            fill="none" />
        </svg>
        <div class="content">
          <br>
          <span>${mining}</span>
          <br>
          <br>
          <!-- <span>
            <label>Foo</label>
            <span>Bar</span><span class="separator"></span><label>Foo</label>
            <span>Bar</span>
          </span> -->
        </div>
      </div>
    `
  }
}

function countStore (state, emitter) {
  state.count = 0
  emitter.on('increment', function (count) {
    state.count += count
    emitter.emit('render')
  })
}
