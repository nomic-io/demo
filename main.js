var html = require('choo/html')
var devtools = require('choo-devtools')
var choo = require('choo')

var app = choo()
app.use(devtools())
app.use(countStore)
app.route('/', mainView)
app.mount('body')

function mainView (state, emit) {
  return html`
    <body>
      ${chainView()}
    </body>
  `
}

function chainView (state, emit) {
  return html`
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
  `
}

function countStore (state, emitter) {
  state.count = 0
  emitter.on('increment', function (count) {
    state.count += count
    emitter.emit('render')
  })
}
