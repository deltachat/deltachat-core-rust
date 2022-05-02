// @ts-check
import DeltaChat, { Message } from '../dist'
import binding from '../binding'

import { strictEqual } from 'assert'
import chai, { expect } from 'chai'
import chaiAsPromised from 'chai-as-promised'
import { EventId2EventName, C } from '../dist/constants'
import { join } from 'path'
import { mkdtempSync, statSync } from 'fs'
import { tmpdir } from 'os'
import { Context } from '../dist/context'
chai.use(chaiAsPromised)

async function createTempUser(url) {
  const fetch = require('node-fetch')

  async function postData(url = '') {
    // Default options are marked with *
    const response = await fetch(url, {
      method: 'POST', // *GET, POST, PUT, DELETE, etc.
      mode: 'cors', // no-cors, *cors, same-origin
      cache: 'no-cache', // *default, no-cache, reload, force-cache, only-if-cached
      credentials: 'same-origin', // include, *same-origin, omit
      headers: {
        'cache-control': 'no-cache',
      },
      referrerPolicy: 'no-referrer', // no-referrer, *client
    })
    return response.json() // parses JSON response into native JavaScript objects
  }

  return await postData(url)
}

describe('static tests', function () {
  it('reverse lookup of events', function () {
    const eventKeys = Object.keys(EventId2EventName).map((k) => Number(k))
    const eventValues = Object.values(EventId2EventName)
    const reverse = eventValues.map((v) => C[v])
    expect(reverse).to.be.deep.equal(eventKeys)
  })

  it('event constants are consistent', function () {
    const eventKeys = Object.keys(C)
      .filter((k) => k.startsWith('DC_EVENT_'))
      .sort()
    const eventValues = Object.values(EventId2EventName).sort()
    expect(eventKeys).to.be.deep.equal(eventValues)
  })

  it('static method maybeValidAddr()', function () {
    expect(DeltaChat.maybeValidAddr(null)).to.equal(false)
    expect(DeltaChat.maybeValidAddr('')).to.equal(false)
    expect(DeltaChat.maybeValidAddr('uuu')).to.equal(false)
    expect(DeltaChat.maybeValidAddr('dd.tt')).to.equal(false)
    expect(DeltaChat.maybeValidAddr('tt.dd@yggmail')).to.equal(true)
    expect(DeltaChat.maybeValidAddr('u@d')).to.equal(true)
    //expect(DeltaChat.maybeValidAddr('u@d.')).to.equal(false)
    //expect(DeltaChat.maybeValidAddr('u@d.t')).to.equal(false)
    //expect(DeltaChat.maybeValidAddr('u@.tt')).to.equal(false)
    expect(DeltaChat.maybeValidAddr('@d.tt')).to.equal(false)
    expect(DeltaChat.maybeValidAddr('user@domain.tld')).to.equal(true)
    expect(DeltaChat.maybeValidAddr('u@d.tt')).to.equal(true)
  })

  it('static getSystemInfo()', function () {
    const info = Context.getSystemInfo()
    expect(info).to.contain.keys([
      'arch',
      'deltachat_core_version',
      'sqlite_version',
    ])
  })

  it('static context.getProviderFromEmail("example@example.com")', function () {
    const provider = DeltaChat.getProviderFromEmail('example@example.com')

    expect(provider).to.deep.equal({
      before_login_hint: "Hush this provider doesn't exist!",
      overview_page: 'https://providers.delta.chat/example-com',
      status: 3,
    })
  })
})

describe('Basic offline Tests', function () {
  it('opens a context', async function () {
    const { dc, context } = DeltaChat.newTemporary()

    strictEqual(context.isConfigured(), false)
    dc.close()
  })

  it('set config', async function () {
    const { dc, context } = DeltaChat.newTemporary()

    context.setConfig('bot', true)
    strictEqual(context.getConfig('bot'), '1')
    context.setConfig('bot', false)
    strictEqual(context.getConfig('bot'), '0')
    context.setConfig('bot', '1')
    strictEqual(context.getConfig('bot'), '1')
    context.setConfig('bot', '0')
    strictEqual(context.getConfig('bot'), '0')
    context.setConfig('bot', 1)
    strictEqual(context.getConfig('bot'), '1')
    context.setConfig('bot', 0)
    strictEqual(context.getConfig('bot'), '0')

    context.setConfig('bot', null)
    strictEqual(context.getConfig('bot'), '')

    strictEqual(context.getConfig('selfstatus'), '')
    context.setConfig('selfstatus', 'hello')
    strictEqual(context.getConfig('selfstatus'), 'hello')
    context.setConfig('selfstatus', '')
    strictEqual(context.getConfig('selfstatus'), '')
    context.setConfig('selfstatus', null)
    strictEqual(context.getConfig('selfstatus'), '')

    dc.close()
  })

  it('configure with either missing addr or missing mail_pw throws', async function () {
    const { dc, context } = DeltaChat.newTemporary()
    dc.startEvents()

    await expect(
      context.configure({ addr: 'delta1@delta.localhost' })
    ).to.eventually.be.rejectedWith('Please enter a password.')
    await expect(context.configure({ mailPw: 'delta1' })).to.eventually.be
      .rejected

    context.stopOngoingProcess()
    dc.close()
  })

  it('context.getInfo()', async function () {
    const { dc, context } = DeltaChat.newTemporary()

    const info = await context.getInfo()
    expect(typeof info).to.be.equal('object')
    expect(info).to.contain.keys([
      'arch',
      'bcc_self',
      'blobdir',
      'bot',
      'configured_mvbox_folder',
      'configured_sentbox_folder',
      'database_dir',
      'database_encrypted',
      'database_version',
      'delete_device_after',
      'delete_server_after',
      'deltachat_core_version',
      'display_name',
      'download_limit',
      'e2ee_enabled',
      'entered_account_settings',
      'fetch_existing_msgs',
      'fingerprint',
      'folders_configured',
      'is_configured',
      'journal_mode',
      'key_gen_type',
      'last_housekeeping',
      'level',
      'mdns_enabled',
      'media_quality',
      'messages_in_contact_requests',
      'mvbox_move',
      'num_cpus',
      'number_of_chat_messages',
      'number_of_chats',
      'number_of_contacts',
      'only_fetch_mvbox',
      'private_key_count',
      'public_key_count',
      'quota_exceeding',
      'scan_all_folders_debounce_secs',
      'selfavatar',
      'send_sync_msgs',
      'sentbox_watch',
      'show_emails',
      'socks5_enabled',
      'sqlite_version',
      'uptime',
      'used_account_settings',
      'webrtc_instance',
    ])

    dc.close()
  })
})

describe('Offline Tests with unconfigured account', function () {
  let [dc, context, accountId, directory] = [null, null, null, null]

  this.beforeEach(async function () {
    let tmp = DeltaChat.newTemporary()
    dc = tmp.dc
    context = tmp.context
    accountId = tmp.accountId
    directory = tmp.directory
    dc.startEvents()
  })

  this.afterEach(async function () {
    if (context) {
      context.stopOngoingProcess()
    }
    if (dc) {
      try {
        dc.stopIO()
        dc.close()
      } catch (error) {
        console.error(error)
      }
    }

    dc = null
    context = null
    accountId = null
    directory = null
  })

  it('invalid context.joinSecurejoin', async function () {
    expect(context.joinSecurejoin('test')).to.be.eq(0)
  })

  it('Device Chat', async function () {
    const deviceChatMessageText = 'test234'

    expect((await context.getChatList(0, '', null)).getCount()).to.equal(
      0,
      'no device chat after setup'
    )

    await context.addDeviceMessage('test', deviceChatMessageText)

    const chatList = await context.getChatList(0, '', null)
    expect(chatList.getCount()).to.equal(
      1,
      'device chat after adding device msg'
    )

    const deviceChatId = await chatList.getChatId(0)
    const deviceChat = await context.getChat(deviceChatId)
    expect(deviceChat.isDeviceTalk()).to.be.true
    expect(deviceChat.toJson().isDeviceTalk).to.be.true

    const deviceChatMessages = await context.getChatMessages(deviceChatId, 0, 0)
    expect(deviceChatMessages.length).to.be.equal(
      1,
      'device chat has added message'
    )

    const deviceChatMessage = await context.getMessage(deviceChatMessages[0])
    expect(deviceChatMessage.getText()).to.equal(
      deviceChatMessageText,
      'device chat message has the inserted text'
    )
  })

  it('should have e2ee enabled and right blobdir', function () {
    expect(context.getConfig('e2ee_enabled')).to.equal(
      '1',
      'e2eeEnabled correct'
    )
    expect(
      String(context.getBlobdir()).startsWith(directory),
      'blobdir should be inside temp directory'
    )
    expect(
      String(context.getBlobdir()).endsWith('db.sqlite-blobs'),
      'blobdir end with "db.sqlite-blobs"'
    )
  })

  it('should create chat from contact and Chat methods', async function () {
    const contactId = context.createContact('aaa', 'aaa@site.org')

    strictEqual(context.lookupContactIdByAddr('aaa@site.org'), contactId)
    strictEqual(context.lookupContactIdByAddr('nope@site.net'), 0)

    let chatId = context.createChatByContactId(contactId)
    let chat = context.getChat(chatId)

    strictEqual(
      chat.getVisibility(),
      C.DC_CHAT_VISIBILITY_NORMAL,
      'not archived'
    )
    strictEqual(chat.getId(), chatId, 'chat id matches')
    strictEqual(chat.getName(), 'aaa', 'chat name matches')
    strictEqual(chat.getProfileImage(), null, 'no profile image')
    strictEqual(chat.getType(), C.DC_CHAT_TYPE_SINGLE, 'single chat')
    strictEqual(chat.isSelfTalk(), false, 'no self talk')
    // TODO make sure this is really the case!
    strictEqual(chat.isUnpromoted(), false, 'not unpromoted')
    strictEqual(chat.isProtected(), false, 'not verified')
    strictEqual(typeof chat.color, 'string', 'color is a string')

    strictEqual(context.getDraft(chatId), null, 'no draft message')
    context.setDraft(chatId, context.messageNew().setText('w00t!'))
    strictEqual(
      context.getDraft(chatId).toJson().text,
      'w00t!',
      'draft text correct'
    )
    context.setDraft(chatId, null)
    strictEqual(context.getDraft(chatId), null, 'draft removed')

    strictEqual(context.getChatIdByContactId(contactId), chatId)
    expect(context.getChatContacts(chatId)).to.deep.equal([contactId])

    context.setChatVisibility(chatId, C.DC_CHAT_VISIBILITY_ARCHIVED)
    strictEqual(
      context.getChat(chatId).getVisibility(),
      C.DC_CHAT_VISIBILITY_ARCHIVED,
      'chat archived'
    )
    context.setChatVisibility(chatId, C.DC_CHAT_VISIBILITY_NORMAL)
    strictEqual(
      chat.getVisibility(),
      C.DC_CHAT_VISIBILITY_NORMAL,
      'chat unarchived'
    )

    chatId = context.createGroupChat('unverified group', false)
    chat = context.getChat(chatId)
    strictEqual(chat.isProtected(), false, 'is not verified')
    strictEqual(chat.getType(), C.DC_CHAT_TYPE_GROUP, 'group chat')
    expect(context.getChatContacts(chatId)).to.deep.equal([
      C.DC_CONTACT_ID_SELF,
    ])

    const draft2 = context.getDraft(chatId)
    expect(draft2 == null, 'unpromoted group has no draft by default')

    context.setChatName(chatId, 'NEW NAME')
    strictEqual(context.getChat(chatId).getName(), 'NEW NAME', 'name updated')

    chatId = context.createGroupChat('a verified group', true)
    chat = context.getChat(chatId)
    strictEqual(chat.isProtected(), true, 'is verified')
  })

  it('test setting profile image', async function () {
    const chatId = context.createGroupChat('testing profile image group', false)
    const image = 'image.jpeg'
    const imagePath = join(__dirname, 'fixtures', image)
    const blobs = context.getBlobdir()

    context.setChatProfileImage(chatId, imagePath)
    const blobPath = context.getChat(chatId).getProfileImage()
    expect(blobPath.startsWith(blobs)).to.be.true
    expect(blobPath.endsWith(image)).to.be.true

    context.setChatProfileImage(chatId, null)
    expect(context.getChat(chatId).getProfileImage()).to.be.equal(
      null,
      'image is null'
    )
  })

  it('test setting ephemeral timer', function () {
    const chatId = context.createGroupChat('testing ephemeral timer')

    strictEqual(
      context.getChatEphemeralTimer(chatId),
      0,
      'ephemeral timer is not set by default'
    )

    context.setChatEphemeralTimer(chatId, 60)
    strictEqual(
      context.getChatEphemeralTimer(chatId),
      60,
      'ephemeral timer is set to 1 minute'
    )

    context.setChatEphemeralTimer(chatId, 0)
    strictEqual(
      context.getChatEphemeralTimer(chatId),
      0,
      'ephemeral timer is reset'
    )
  })

  it('should create and delete chat', function () {
    const chatId = context.createGroupChat('GROUPCHAT')
    const chat = context.getChat(chatId)
    strictEqual(chat.getId(), chatId, 'correct chatId')
    context.deleteChat(chat.getId())
    strictEqual(context.getChat(chatId), null, 'chat removed')
  })

  it('new message and Message methods', function () {
    const text = 'w00t!'
    const msg = context.messageNew().setText(text)

    strictEqual(msg.getChatId(), 0, 'chat id 0 before sent')
    strictEqual(msg.getDuration(), 0, 'duration 0 before sent')
    strictEqual(msg.getFile(), '', 'no file set by default')
    strictEqual(msg.getFilebytes(), 0, 'and file bytes is 0')
    strictEqual(msg.getFilemime(), '', 'no filemime by default')
    strictEqual(msg.getFilename(), '', 'no filename set by default')
    strictEqual(msg.getFromId(), 0, 'no contact id set by default')
    strictEqual(msg.getHeight(), 0, 'plain text message have height 0')
    strictEqual(msg.getId(), 0, 'id 0 before sent')
    strictEqual(msg.getSetupcodebegin(), '', 'no setupcode begin')
    strictEqual(msg.getShowpadlock(), false, 'no padlock by default')

    const state = msg.getState()
    strictEqual(state.isUndefined(), true, 'no state by default')
    strictEqual(state.isFresh(), false, 'no state by default')
    strictEqual(state.isNoticed(), false, 'no state by default')
    strictEqual(state.isSeen(), false, 'no state by default')
    strictEqual(state.isPending(), false, 'no state by default')
    strictEqual(state.isFailed(), false, 'no state by default')
    strictEqual(state.isDelivered(), false, 'no state by default')
    strictEqual(state.isReceived(), false, 'no state by default')

    const summary = msg.getSummary()
    strictEqual(summary.getId(), 0, 'no summary id')
    strictEqual(summary.getState(), 0, 'no summary state')
    strictEqual(summary.getText1(), null, 'no summary text1')
    strictEqual(summary.getText1Meaning(), 0, 'no summary text1 meaning')
    strictEqual(summary.getText2(), '', 'no summary text2')
    strictEqual(summary.getTimestamp(), 0, 'no summary timestamp')

    //strictEqual(msg.getSummarytext(50), text, 'summary text is text')
    strictEqual(msg.getText(), text, 'msg text set correctly')
    strictEqual(msg.getTimestamp(), 0, 'no timestamp')

    const viewType = msg.getViewType()
    strictEqual(viewType.isText(), true)
    strictEqual(viewType.isImage(), false)
    strictEqual(viewType.isGif(), false)
    strictEqual(viewType.isAudio(), false)
    strictEqual(viewType.isVoice(), false)
    strictEqual(viewType.isVideo(), false)
    strictEqual(viewType.isFile(), false)

    strictEqual(msg.getWidth(), 0, 'no message width')
    strictEqual(msg.isDeadDrop(), false, 'not deaddrop')
    strictEqual(msg.isForwarded(), false, 'not forwarded')
    strictEqual(msg.isIncreation(), false, 'not in creation')
    strictEqual(msg.isInfo(), false, 'not an info message')
    strictEqual(msg.isSent(), false, 'messge is not sent')
    strictEqual(msg.isSetupmessage(), false, 'not an autocrypt setup message')

    msg.latefilingMediasize(10, 20, 30)
    strictEqual(msg.getWidth(), 10, 'message width set correctly')
    strictEqual(msg.getHeight(), 20, 'message height set correctly')
    strictEqual(msg.getDuration(), 30, 'message duration set correctly')

    msg.setDimension(100, 200)
    strictEqual(msg.getWidth(), 100, 'message width set correctly')
    strictEqual(msg.getHeight(), 200, 'message height set correctly')

    msg.setDuration(314)
    strictEqual(msg.getDuration(), 314, 'message duration set correctly')

    expect(() => {
      msg.setFile(null)
    }).to.throw('Missing filename')

    const logo = join(__dirname, 'fixtures', 'logo.png')
    const stat = statSync(logo)
    msg.setFile(logo)
    strictEqual(msg.getFilebytes(), stat.size, 'correct file size')
    strictEqual(msg.getFile(), logo, 'correct file name')
    strictEqual(msg.getFilemime(), 'image/png', 'mime set implicitly')
    msg.setFile(logo, 'image/gif')
    strictEqual(msg.getFilemime(), 'image/gif', 'mime set (in)correctly')
    msg.setFile(logo, 'image/png')
    strictEqual(msg.getFilemime(), 'image/png', 'mime set correctly')

    const json = msg.toJson()
    expect(json).to.not.equal(null, 'not null')
    strictEqual(typeof json, 'object', 'json object')
  })

  it('Contact methods', function () {
    const contactId = context.createContact('First Last', 'first.last@site.org')
    const contact = context.getContact(contactId)

    strictEqual(contact.getAddress(), 'first.last@site.org', 'correct address')
    strictEqual(typeof contact.color, 'string', 'color is a string')
    strictEqual(contact.getDisplayName(), 'First Last', 'correct display name')
    strictEqual(contact.getId(), contactId, 'contact id matches')
    strictEqual(contact.getName(), 'First Last', 'correct name')
    strictEqual(contact.getNameAndAddress(), 'First Last (first.last@site.org)')
    strictEqual(contact.getProfileImage(), null, 'no contact image')
    strictEqual(contact.isBlocked(), false, 'not blocked')
    strictEqual(contact.isVerified(), false, 'unverified status')
    strictEqual(contact.lastSeen, 0, 'last seen unknown')
  })

  it('create contacts from address book', function () {
    const addresses = [
      'Name One',
      'name1@site.org',
      'Name Two',
      'name2@site.org',
      'Name Three',
      'name3@site.org',
    ]
    const count = context.addAddressBook(addresses.join('\n'))
    strictEqual(count, addresses.length / 2)
    context
      .getContacts(0, 'Name ')
      .map((id) => context.getContact(id))
      .forEach((contact) => {
        expect(contact.getName().startsWith('Name ')).to.be.true
      })
  })

  it('delete contacts', function () {
    const id = context.createContact('someuser', 'someuser@site.com')
    const contact = context.getContact(id)
    strictEqual(contact.getId(), id, 'contact id matches')
    strictEqual(context.deleteContact(id), true, 'delete call succesful')
    strictEqual(context.getContact(id), null, 'contact is gone')
  })

  it('adding and removing a contact from a chat', function () {
    const chatId = context.createGroupChat('adding_and_removing')
    const contactId = context.createContact('Add Remove', 'add.remove@site.com')
    strictEqual(
      context.addContactToChat(chatId, contactId),
      true,
      'contact added'
    )
    strictEqual(
      context.isContactInChat(chatId, contactId),
      true,
      'contact in chat'
    )
    strictEqual(
      context.removeContactFromChat(chatId, contactId),
      true,
      'contact removed'
    )
    strictEqual(
      context.isContactInChat(chatId, contactId),
      false,
      'contact not in chat'
    )
  })

  it('blocking contacts', function () {
    const id = context.createContact('badcontact', 'bad@site.com')

    strictEqual(context.getBlockedCount(), 0)
    strictEqual(context.getContact(id).isBlocked(), false)
    expect(context.getBlockedContacts()).to.be.empty

    context.blockContact(id, true)
    strictEqual(context.getBlockedCount(), 1)
    strictEqual(context.getContact(id).isBlocked(), true)
    expect(context.getBlockedContacts()).to.deep.equal([id])

    context.blockContact(id, false)
    strictEqual(context.getBlockedCount(), 0)
    strictEqual(context.getContact(id).isBlocked(), false)
    expect(context.getBlockedContacts()).to.be.empty
  })

  it('ChatList methods', function () {
    const ids = [
      context.createGroupChat('groupchat1'),
      context.createGroupChat('groupchat11'),
      context.createGroupChat('groupchat111'),
    ]

    let chatList = context.getChatList(0, 'groupchat1', null)
    strictEqual(chatList.getCount(), 3, 'should contain above chats')
    expect(ids.indexOf(chatList.getChatId(0))).not.to.equal(-1)
    expect(ids.indexOf(chatList.getChatId(1))).not.to.equal(-1)
    expect(ids.indexOf(chatList.getChatId(2))).not.to.equal(-1)

    const lot = chatList.getSummary(0)
    strictEqual(lot.getId(), 0, 'lot has no id')
    strictEqual(lot.getState(), C.DC_STATE_UNDEFINED, 'correct state')

    const text = 'No messages.'
    context.createGroupChat('groupchat1111')
    chatList = context.getChatList(0, 'groupchat1111', null)
    strictEqual(
      chatList.getSummary(0).getText2(),
      text,
      'custom new group message'
    )

    context.setChatVisibility(ids[0], C.DC_CHAT_VISIBILITY_ARCHIVED)
    chatList = context.getChatList(C.DC_GCL_ARCHIVED_ONLY, 'groupchat1', null)
    strictEqual(chatList.getCount(), 1, 'only one archived')
  })

  it('Remove qoute from (draft) message', function () {
    context.addDeviceMessage('test_qoute', 'test')
    const msgId = context.getChatMessages(10, 0, 0)[0]
    const msg = context.messageNew()

    msg.setQuote(context.getMessage(msgId))
    expect(msg.getQuotedMessage()).to.not.be.null
    msg.setQuote(null)
    expect(msg.getQuotedMessage()).to.be.null
  })
})

describe('Integration tests', function () {
  this.timeout(60 * 3000) // increase timeout to 1min

  let [dc, context, accountId, directory, account] = [
    null,
    null,
    null,
    null,
    null,
  ]

  let [dc2, context2, accountId2, directory2, account2] = [
    null,
    null,
    null,
    null,
    null,
  ]

  this.beforeEach(async function () {
    let tmp = DeltaChat.newTemporary()
    dc = tmp.dc
    context = tmp.context
    accountId = tmp.accountId
    directory = tmp.directory
    dc.startEvents()
  })

  this.afterEach(async function () {
    if (context) {
      try {
        context.stopOngoingProcess()
      } catch (error) {
        console.error(error)
      }
    }
    if (context2) {
      try {
        context2.stopOngoingProcess()
      } catch (error) {
        console.error(error)
      }
    }

    if (dc) {
      try {
        dc.stopIO()
        dc.close()
      } catch (error) {
        console.error(error)
      }
    }

    dc = null
    context = null
    accountId = null
    directory = null

    context2 = null
    accountId2 = null
    directory2 = null
  })

  this.beforeAll(async function () {
    if (!process.env.DCC_NEW_TMP_EMAIL) {
      console.log(
        'Missing DCC_NEW_TMP_EMAIL environment variable!, skip intergration tests'
      )
      this.skip()
    }

    account = await createTempUser(process.env.DCC_NEW_TMP_EMAIL)
    if (!account || !account.email || !account.password) {
      console.log(
        "We didn't got back an account from the api, skip intergration tests"
      )
      this.skip()
    }
  })

  it('configure', async function () {
    strictEqual(context.isConfigured(), false, 'should not be configured')

    // Not sure what's the best way to check the events
    // TODO: check the events

    // dc.once('DC_EVENT_CONFIGURE_PROGRESS', (data) => {
    //   t.pass('DC_EVENT_CONFIGURE_PROGRESS called at least once')
    // })
    // dc.on('DC_EVENT_ERROR', (error) => {
    //   console.error('DC_EVENT_ERROR', error)
    // })
    // dc.on('DC_EVENT_ERROR_NETWORK', (first, error) => {
    //   console.error('DC_EVENT_ERROR_NETWORK', error)
    // })

    // dc.on('ALL', (event, data1, data2) => console.log('ALL', event, data1, data2))

    await expect(
      context.configure({
        addr: account.email,
        mail_pw: account.password,

        displayname: 'Delta One',
        selfstatus: 'From Delta One with <3',
        selfavatar: join(__dirname, 'fixtures', 'avatar.png'),
      })
    ).to.be.eventually.fulfilled

    strictEqual(context.getConfig('addr'), account.email, 'addr correct')
    strictEqual(
      context.getConfig('displayname'),
      'Delta One',
      'displayName correct'
    )
    strictEqual(
      context.getConfig('selfstatus'),
      'From Delta One with <3',
      'selfStatus correct'
    )
    expect(
      context.getConfig('selfavatar').endsWith('avatar.png'),
      'selfavatar correct'
    )
    strictEqual(context.getConfig('e2ee_enabled'), '1', 'e2ee_enabled correct')
    strictEqual(context.getConfig('sentbox_watch'), '0', 'sentbox_watch')
    strictEqual(context.getConfig('mvbox_move'), '0', 'mvbox_move')
    strictEqual(
      context.getConfig('save_mime_headers'),
      '',
      'save_mime_headers correct'
    )

    expect(context.getBlobdir().endsWith('db.sqlite-blobs'), 'correct blobdir')
    strictEqual(context.isConfigured(), true, 'is configured')

    // whole re-configure to only change displayname: what the heck? (copied this from the old test)
    await expect(
      context.configure({
        addr: account.email,
        mail_pw: account.password,
        displayname: 'Delta Two',
        selfstatus: 'From Delta One with <3',
        selfavatar: join(__dirname, 'fixtures', 'avatar.png'),
      })
    ).to.be.eventually.fulfilled
    strictEqual(
      context.getConfig('displayname'),
      'Delta Two',
      'updated displayName correct'
    )
  })

  it('Autocrypt setup - key transfer', async function () {
    // Spawn a second dc instance with same account
    // dc.on('ALL', (event, data1, data2) =>
    //   console.log('FIRST ', event, data1, data2)
    // )
    dc.stopIO()
    await expect(
      context.configure({
        addr: account.email,
        mail_pw: account.password,

        displayname: 'Delta One',
        selfstatus: 'From Delta One with <3',
        selfavatar: join(__dirname, 'fixtures', 'avatar.png'),
      })
    ).to.be.eventually.fulfilled

    const accountId2 = dc.addAccount()
    console.log('accountId2:', accountId2)
    context2 = dc.accountContext(accountId2)

    let setupCode = null
    const waitForSetupCode = waitForSomething()
    const waitForEnd = waitForSomething()

    dc.on('ALL', (event, accountId, data1, data2) => {
      console.log('[' + accountId + ']', event, data1, data2)
    })

    dc.on('DC_EVENT_MSGS_CHANGED', async (aId, chatId, msgId) => {
      console.log('[' + accountId + '] DC_EVENT_MSGS_CHANGED', chatId, msgId)
      if (
        aId != accountId ||
        !context.getChat(chatId).isSelfTalk() ||
        !context.getMessage(msgId).isSetupmessage()
      ) {
        return
      }
      console.log('Setupcode!')
      let setupCode = await waitForSetupCode.promise
      // console.log('incoming msg', { setupCode })
      const messages = context.getChatMessages(chatId, 0, 0)
      expect(messages.indexOf(msgId) !== -1, 'msgId is in chat messages').to.be
        .true
      const result = await context.continueKeyTransfer(msgId, setupCode)
      expect(result === true, 'continueKeyTransfer was successful').to.be.true

      waitForEnd.done()
    })

    dc.stopIO()
    await expect(
      context2.configure({
        addr: account.email,
        mail_pw: account.password,

        displayname: 'Delta One',
        selfstatus: 'From Delta One with <3',
        selfavatar: join(__dirname, 'fixtures', 'avatar.png'),
      })
    ).to.be.eventually.fulfilled
    dc.startIO()

    console.log('Sending autocrypt setup code')
    setupCode = await context2.initiateKeyTransfer()
    console.log('Sent autocrypt setup code')
    waitForSetupCode.done(setupCode)
    console.log('setupCode is: ' + setupCode)
    expect(typeof setupCode).to.equal('string', 'setupCode is string')

    await waitForEnd.promise
  })

  it('configure using invalid password should fail', async function () {
    await expect(
      context.configure({
        addr: 'hpk5@testrun.org',
        mail_pw: 'asd',
      })
    ).to.be.eventually.rejected
  })
})

/**
 * @returns {{done: (result?)=>void, promise:Promise<any> }}
 */
function waitForSomething() {
  let resolvePromise
  const promise = new Promise((res, rej) => {
    resolvePromise = res
  })
  return {
    done: resolvePromise,
    promise,
  }
}
