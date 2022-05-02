//@ts-check
const { Context } = require('../dist')

const opts = {
  addr: '[email]',
  mail_pw: '[password]',
}

const contact = '[email]'

async function main() {
  const dc = Context.open('./')
  dc.on('ALL', console.log.bind(null, 'core |'))

  try {
    await dc.configure(opts)
  } catch (err) {
    console.error('Failed to configure because of: ', err)
    dc.unref()
    return
  }

  dc.startIO()
  console.log('fully configured')

  const contactId = dc.createContact('Test', contact)
  const chatId = dc.createChatByContactId(contactId)
  dc.sendMessage(chatId, 'Hi!')

  console.log('sent message')

  dc.once('DC_EVENT_SMTP_MESSAGE_SENT', async () => {
    console.log('Message sent, shutting down...')
    dc.stopIO()
    console.log('stopped io')
    dc.unref()
  })
}

main()
