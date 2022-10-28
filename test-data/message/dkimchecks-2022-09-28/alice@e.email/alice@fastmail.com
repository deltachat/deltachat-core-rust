From: <alice@fastmail.com>
To: <alice@e.email>
Authentication-Results: mail2.ecloud.global;
	dkim=pass header.d=fastmail.com header.s=fm2 header.b=bQ080jJU;
	dkim=pass header.d=messagingengine.com header.s=fm2 header.b=FVyMuSGb;
	dmarc=pass (policy=none) header.from=fastmail.com;
	spf=pass (mail2.ecloud.global: domain of alice@fastmail.com designates 66.111.4.28 as permitted sender) smtp.mailfrom=alice@fastmail.com
