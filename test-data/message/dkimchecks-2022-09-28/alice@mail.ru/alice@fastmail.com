From: <alice@fastmail.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx285.i.mail.ru: domain of fastmail.com designates 66.111.4.28 as permitted sender) smtp.mailfrom=alice@fastmail.com smtp.helo=out4-smtp.messagingengine.com;
	 dkim=pass header.d=fastmail.com;
	 dkim=pass header.d=messagingengine.com; dmarc=pass header.from=alice@fastmail.com
