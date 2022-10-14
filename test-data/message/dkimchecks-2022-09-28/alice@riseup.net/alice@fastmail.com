Authentication-Results: mx1.riseup.net;
	dkim=pass (2048-bit key; unprotected) header.d=fastmail.com header.i=@fastmail.com header.a=rsa-sha256 header.s=fm2 header.b=ZyZhU7V7;
	dkim=pass (2048-bit key; unprotected) header.d=messagingengine.com header.i=@messagingengine.com header.a=rsa-sha256 header.s=fm2 header.b=GQt3UCVa;
	dkim-atps=neutral
From: <alice@fastmail.com>
To: bot <alice@riseup.net>
