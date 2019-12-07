let html = require('choo/html')
let devtools = require('choo-devtools')
let choo = require('choo')
let logo = require('./logo.png')
let timeago = require('timeago.js').format

require('timeago.js').register('en_short', function (number, index) {
  return [
    ['1s ago', 'right now'],
    ['%ss ago', 'in %ss'],
    ['1m ago', 'in 1m'],
    ['%sm ago', 'in %sm'],
    ['1h ago', 'in 1h'],
    ['%sh ago', 'in %sh'],
    ['1d ago', 'in 1d'],
    ['%sd ago', 'in %sd'],
    ['1w ago', 'in 1w'],
    ['%sw ago', 'in %sw'],
    ['1mo ago', 'in 1mo'],
    ['%smo ago', 'in %smo'],
    ['1yr ago', 'in 1yr'],
    ['%syr ago', 'in %syr'],
  ][index]
})

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
  

  console.log('chains', state.first.blocks, state.second.blocks)

  let block = (b) => blockView(b, emit)
  let linkAndBlock = (b) => b ? [
    html`<div class="link ${b.fade ? 'fade' : ''} ${(b.mining || b.drop) ? 'mining' : ''}"></div>`,
    block(b)
  ] : []

  let firstBlocks = state.first.blocks.slice()
  let secondBlocks = state.second.blocks.slice()
  let i = 0
  while (firstBlocks.length + secondBlocks.length > 0) {
    i += 1
    if (i > 100) break

    let first = firstBlocks[0]
    let second = secondBlocks[0]
    
    if (second && !second.drop && second.link) {
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
    } else if (firstBlocks.length > 1 && secondBlocks.length === 1) {
      // ran out of second chain blocks but still have first, add skip space
      items.first.push(...linkAndBlock(firstBlocks.shift()))
      items.inter.push(html`<div class="block w"></div>`)
      items.second.push(html`<div class="link skip ${(second.mining || second.drop) ? 'mining' : ''}"></div>`)
    } else if (secondBlocks.length > 1 && firstBlocks.length === 1) {
      // ran out of first chain blocks but still have second, add skip space
      items.first.push(html`<div class="link skip ${(first.mining || first.drop) ? 'mining' : ''}"></div>`)
      items.inter.push(html`<div class="block z"></div>`)
      items.second.push(...linkAndBlock(secondBlocks.shift()))
    } else if (first && second && second.link !== first.height && !first.mining && !second.mining) {
      // first isn't linked by second, add some skip space
      items.first.push(html`<div class="link skip ${(first.mining || first.drop) ? 'mining' : ''} expand"></div>`)
      items.inter.push(html`<div class="block x"></div>`)
      items.second.push(...linkAndBlock(secondBlocks.shift()))
    } else {
      // first isn't linked to, show it next to second
      items.first.push(...linkAndBlock(firstBlocks.shift()))
      items.inter.push(html`<div class="block y"></div>`)
      items.second.push(...linkAndBlock(secondBlocks.shift()))
    }
  }

  return html`
    <section class="chains">
      <div class="chain">
        <label><span>${state.first.name}</span></label>
        <!--<div class="fade"><div class="link"></div></div>
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

          <br>
          <span class="ago">${timeago(b.time, 'en_short')}</span>
          <span class="icon"></span>

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
    let w = 110 - r - 3 * 2
    let h = 80 - r - 3 * 2

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
    if (e.animationName === 'drop' || e.animationName === 'drop-secondary') {
      setTimeout(() => {
        link.classList.remove('mining')
        if (link.previousElementSibling.classList.contains('link')) {
          link.previousElementSibling.classList.remove('mining')
        console.log('!!!!\n\n!!!!', link.previousElementSibling)
        }
      }, 530)
    } else if (e.animationName === 'fold') {
      link.classList.add('folding')
    } else if (e.animationName === 'fade') {
      link.classList.add('fade')
    }
  })

  el.addEventListener('animationend', (e) => {
    console.log('end', e)
    if (e.animationName === 'drop' || e.animationName === 'drop-secondary') {
      e.target.classList.remove('drop')
      if (e.target.classList.contains('fold')) {
        e.target.classList.remove('fold')
        e.target.classList.add('folding')
      } else {
        if (e.target.parentNode.classList.contains('secondary')) {
          emit('fold-second')
        } else {
          emit('fold-first')
        }
      }
    } else if (e.animationName === 'fold') {
      if (e.target.parentNode.classList.contains('secondary')) {
        emit('fold-second')
      } else {
        emit('fold-first')
      }
    } else if (e.animationName === 'fade') {
      e.target.previousElementSibling.classList.remove('fade')
      e.target.classList.remove('fade')
      delete b.fade
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
  let queues = {
    first: [],
    second: []
  }


  let lastLink = null

  let initialized = false
  function init () {
    initialized = false
    lastLink = null
    queues = {
      first: [],
      second: []
    }
    state.chains = {
      first: {
        name: 'Bitcoin Testnet',
        blocks: [
          // { height: 100000, hash: '0000000000123456789' },
          // { height: 100001, hash: '0000000000123456789' },
          { mining: true }
        ]
      },
      second: {
        name: 'Nomic Sidechain Testnet',
        blocks: [
          // { height: 1000000, hash: '0000000000123456789', link: 100000 },
          // { stack: 100, height: 1000001, hash: '0000000000123456789' },
          // { height: 1000002, hash: '0000000000123456789', link: 100001 },
          // { height: 1000003, stack: 100, hash: 'f000ab12cd0123456789' },
          { mining: true }
        ]
      }
    }
  }

  function push (chain) {
    return function (block) {
      let blocks = state.chains[chain].blocks
      let last = blocks[blocks.length - 1]

      if (last.mining) {
        blocks.pop()
      }
      if (chain === 'second') {
        let last = blocks[blocks.length - 1]
        if (last.link == null && block.link == null) {
          block.fold = true
        }
        console.log('push', last, block)
      }

      if (!block.mining) {
        block.drop = true
      }
      blocks.push(block)

      emitter.emit('shift')
      emitter.emit('render')
    }
  }
  emitter.on('push-first', push('first'))
  emitter.on('push-second', push('second'))

  function fold (chain) {
    return function () {
      let blocks = state.chains[chain].blocks

      let last = blocks.pop()
      let under = blocks.pop()

      delete last.drop
      delete last.fold

      if (chain === 'first') {
        blocks.push(under)
        blocks.push(last)
      } else if (chain === 'second') {
        if (under.link != null || last.link != null) {
          blocks.push(under)
          blocks.push(last)
        } else {
          last.stack = last.height - blocks[blocks.length - 1].height
          last.fold = true
          blocks.push(last)
        }
      }

      blocks.push({ mining: true, fade: true })

      emitter.emit('render')
    }
  }
  emitter.on('fold-first', fold('first'))
  emitter.on('fold-second', fold('second'))

  emitter.on('shift', function () {
    while (state.chains.second.blocks.length > 5
      || state.chains.second.blocks[0].link == null) {
      let shifted = state.chains.second.blocks.shift()
      if (shifted.link != null) {
        while (state.chains.first.blocks[0].height <= shifted.link) {
          state.chains.first.blocks.shift()
        }
      }
      if (state.chains.second.blocks.length > 0) {
        delete state.chains.second.blocks[0].stack
      }
    }
  })

  // setTimeout(() => {
  //   emitter.emit('push-second', { height: 1234, hash: 'asdfasdfasf' })
  // }, 1000)

  // setTimeout(() => {
  //   emitter.emit('push-first', { height: 100002, hash: 'asdfasdfasf' })
  // }, 3000)

  // setTimeout(() => {
  //   emitter.emit('push-second', { height: 1234, hash: 'asdfasdfasf', link: 100002 })
  //   setInterval(() =>
  //     emitter.emit('push-second', { height: Math.random() * 100000 | 0, hash: 'asdfasdfasf' }),
  //     5000
  //   )
  // }, 5000)

  let ws = new WebSocket('ws://localhost:8080')
  ws.onmessage = function ({ data }) {
    data = JSON.parse(data)
    console.log('<<', data)

    if (!initialized) {
      initialized = true

      state.chains.first.blocks.unshift(...data.bitcoin.map(formatBtcBlock))
      let tmBlocks = data.tendermint.map(formatTmBlock)
        .sort((a, b) => a.height - b.height) // ?
        .map((block) => {
          if (!block.hasHeaderTx) return block
        
          if (lastLink == null) {
            block.link = state.chains.first.blocks[0].height
          } else {
            block.link = lastLink + 1
          }
          lastLink = block.link
          
          return block
        })

      state.chains.second.blocks.unshift(...tmBlocks)
      let prev = null
      for (let block of state.chains.second.blocks) {
        if (prev != null) {
          block.stack = block.height - prev.height
          if (block.stack == 1) delete block.stack
        }
        prev = block
      }
      emitter.emit('shift')
      emitter.emit('render')

      setInterval(function () {
        if (queues.first.length > 0) {
          let last = queues.first[queues.first.length - 1]
          queues.first = []
          emitter.emit('push-first', formatBtcBlock(last))
          return
        }
        if (queues.second.length > 0) {
          let last = queues.second[queues.second.length - 1]

          if (last.hasHeaderTx) {
            if (lastLink == null) {
              block.link = state.chains.first.blocks[0].height
            } else {
              block.link = lastLink + 1
            }
            lastLink = block.link
          }

          queues.second = []
          emitter.emit('push-second', formatTmBlock(last))
          return
        }
      }, 2000)

      return
    }

    queues.first.push(...data.bitcoin)
    queues.second.push(...data.tendermint)
  }

  function formatBtcBlock (block) {
    return {
      height: block.height,
      hash: block.hash,
      time: new Date(block.time * 1000)
    }
  }

  function formatTmBlock (block) {
    let hasHeaderTx = (block.data.txs || []).some((txBase64) => {
      let txJson = atob(txBase64)
      return txJson.slice(2, 8) === 'Header'
    })

    return {
      height: Number(block.header.height),
      hash: block.header.last_commit_hash, // :P
      time: new Date(block.header.time),
      hasHeaderTx
    }
  }

  init()
  window.addEventListener('focus', () => window.location = window.location)
}
