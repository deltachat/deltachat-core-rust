Authentication-Results: posteo.de; dmarc=pass (p=none dis=none) header.from=fastmail.com
Authentication-Results: posteo.de; spf=pass smtp.mailfrom=fastmail.com
Authentication-Results: posteo.de;
	dkim=pass (2048-bit key) header.d=fastmail.com header.i=@fastmail.com header.b=tuorMG/I;
	dkim=pass (2048-bit key) header.d=messagingengine.com header.i=@messagingengine.com header.b=mwBYuGTq;
	dkim-atps=neutral
From: <alice@fastmail.com>
To: <alice@posteo.de>
