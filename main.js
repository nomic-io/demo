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
        <div class="block"></div>
        <div class="link"></div>
        <div class="block"></div>
        <div class="link"></div>
        <div class="block">
          <div class="content">
            <span>#654,321</span>
          </div>
        </div>
      </div>
      <div class="chain secondary">
        <div class="block"></div>
        <div class="link"></div>
        <div class="block"></div>
        <div class="link"></div>
        <div class="block">
          <div class="content">
            <span>#654,321</span>
          </div>
        </div>
        <label><span>Nomic Sidechain Testnet</span></label>
      </div>
    </section>
  `
}

function countStore (state, emitter) {
  state.count = 0
  emitter.on('increment', function (count) {
    state.count += count
    emitter.emit('render')
  })
}
