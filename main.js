let html = require('choo/html')
let devtools = require('choo-devtools')
let choo = require('choo')
let logo = require('./logo.png')

let app = choo()
app.use(devtools())
app.use(chainStore)
app.route('/', mainView)
app.mount('body')

function mainView (state, emit) {
  return html`
    <body>
      <img class="logo" src=${logo} />
      ${chainView(state.chains, emit)}
    </body>
  `
}

function chainView (state, emit) {
  let items = {
    first: [],
    inter: [],
    second: []
  }

  let block = (b) => blockView(b, emit)
  let linkAndBlock = (b) => [
    html`<div class="link ${b.fade ? 'fade' : ''}"></div>`,
    block(b)
  ]

  let firstBlocks = state.first.blocks.slice()
  let secondBlocks = state.second.blocks.slice()
  let i = 0
  while (firstBlocks.length + secondBlocks.length > 0) {
    i += 1
    if (i > 100) break

    let first = firstBlocks[0]
    let second = secondBlocks[0]
    
    if (second.link) {
      if (first.height === second.link) {
        // second links to first
        items.first.push(...linkAndBlock(firstBlocks.shift()))
        items.inter.push(html`<div class="link"></div>`)
        items.second.push(...linkAndBlock(secondBlocks.shift()))
      } else if (first.height < second.link) {
        // second links to descendant of first, skip
        items.first.push(...linkAndBlock(firstBlocks.shift()))
        items.inter.push(html`<div class="block"></div>`)
      } else if (first.height > second.link) {
        // second links to ancestor of first, error
        throw Error('cannot link to ancestor')
      }
    } else {
      // check if our descendant links to first block
      let linkedByDescendant = !!secondBlocks
        .slice(1)
        .find(desc => desc.link === first.height)

      if (linkedByDescendant) {
        // first is linked to by our descendant, add some skip space
        items.first.push(html`<div class="link skip"></div>`)
        items.inter.push(html`<div class="block"></div>`)
        items.second.push(...linkAndBlock(secondBlocks.shift()))
      } else {
        // first isn't linked to, show it next to second
        items.first.push(...linkAndBlock(firstBlocks.shift()))
        items.inter.push(html`<div class="block"></div>`)
        items.second.push(...linkAndBlock(secondBlocks.shift()))
      }
    }
  }

  return html`
    <section class="chains">
      <div class="chain">
        <label><span>${state.first.name}</span></label>
        <!--<div class="fade"><div class="link"></div></div>
        ${state.first.blocks.map(() => {})}
        <div class="link skip"></div>
        <div class="link skip mining"></div>
        ${block({ mining: 'Mining' })}-->
        ${items.first}
      </div>
      <div class="interchain">
        <!-- <div class="link"></div>
        <div class="block"></div>
        <div class="link"></div>
        <div class="block"></div>
        <div class="block"></div> -->
        ${items.inter}
      </div>
      <div class="chain secondary">
        <!-- <div class="fade"><div class="link"></div></div> -->
        ${items.second}
        <label><span>${state.second.name}</span></label>
      </div>
    </section>
  `
}

function blockView (b, emit) {
  let el
  if (!b.mining) {
    el = html`
      <div class="block ${`stack${Math.min(3, b.stack)}`} ${b.drop ? 'drop' : ''} ${b.fold ? 'fold' : ''}">
        <div class="content">
          <span>#${b.height.toLocaleString()}</span>
          <br>
          <span class="hash">${truncateHash(b.hash)}</span>
          <br>
          <br>
          <!-- <span>
            <label>Foo</label>
            <span>Bar</span><span class="separator"></span><label>Foo</label>
            <span>Bar</span>
          </span> -->
          ${b.stack ? 
            html`
              <span class="stack-count">
                ${b.stack}
              </span>
            ` : null
          }
        </div>
      </div>
    `
  } else {
    let r = 8
    let w = 90 - r - 3 * 2
    let h = 70 - r - 3 * 2

    let path = `
      m${r+1},2
      h${w}
      a${r},${r} 0 0 1 ${r},${r}
      v${h}
      a${r},${r} 0 0 1 -${r},${r}
      h-${w}
      a${r},${r} 0 0 1 -${r},-${r}
      v-${h}
      a${r},${r} 0 0 1 ${r},-${r}
    `

    el = html`
      <div class="block mining ${b.fade ? 'fade' : ''}">
        <svg width="100%" height="100%">
          <path
            d=${path}
            stroke="#ddd"
            stroke-width="3"
            fill="none" />
        </svg>
        <div class="content">
          <br>
          <span class="mining"></span>
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

  el.addEventListener('animationstart', (e) => {
    let link = e.target.previousElementSibling
    if (e.animationName === 'fold') {
      link.classList.add('folding')
    } else if (e.animationName === 'fade') {
      console.log('fading in')
      link.classList.add('fade')
    }
  })

  el.addEventListener('animationend', (e) => {
    if (e.animationName === 'drop') {
      console.log('block dropped.', e)

      if (e.target.classList.contains('fold')) {
        e.target.classList.remove('drop')
        e.target.classList.remove('fold')
        e.target.classList.add('folding')
        console.log('folding')
      }
    } else if (e.animationName === 'fold') {
      console.log('folded', e)
      emit('fold-second')
    } else if (e.animationName === 'fade') {
      e.target.previousElementSibling.classList.remove('fade')
      e.target.classList.remove('fade')
    }
  })

  return el
}

function truncateHash (hash) {
  if (hash.startsWith('0000')) {
    for (let i = 2; i < hash.length / 2; i++) {
      let byte = hash.slice(i * 2, i * 2 + 2)
      if (byte !== '00') {
        return `00..${hash.slice(i * 2, i * 2 + 6)}`
      }
    }
  } else {
    return hash.slice(0, 10)
  }
}

function chainStore (state, emitter) {
  state.chains = {
    first: {
      name: 'Bitcoin Testnet',
      blocks: [
        { height: 100000, hash: '0000000000123456789' },
        { height: 100001, hash: '0000000000123456789' },
        { mining: true }
      ]
    },
    second: {
      name: 'Nomic Sidechain Testnet',
      blocks: [
        { height: 1000000, hash: '0000000000123456789', link: 100000 },
        { stack: 100, height: 1000001, hash: '0000000000123456789' },
        { height: 1000002, hash: '0000000000123456789', link: 100001 },
        { stack: 50, height: 1000003, hash: 'f000ab12cd0123456789' },
        { mining: true }
      ]
    }
  }

  emitter.on('push-second', function (block) {
    let blocks = state.chains.second.blocks
    let last = blocks[blocks.length - 1]

    if (last.mining) {
      blocks.pop()
      block.drop = true
      blocks.push(block)
      emitter.emit('render')
    }
  })

  emitter.on('fold-second', function () {
    let blocks = state.chains.second.blocks

    let last = blocks.pop()
    let under = blocks.pop()

    last.stack = (under.stack || 1) + 1
    delete last.drop
    delete last.fold
    blocks.push(last)

    blocks.push({ mining: 'Validating', fade: true })

    emitter.emit('render')
  })

  setInterval(() => {
    emitter.emit('push-second', { height: 1234, hash: 'asdfasdfasf', fold: true })
  }, 2000)
}
