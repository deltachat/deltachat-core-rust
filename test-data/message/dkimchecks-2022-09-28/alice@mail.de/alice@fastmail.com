Authentication-Results: mxpostfix02.mail.de;
	dkim=pass (2048-bit key; unprotected) header.d=fastmail.com header.i=@fastmail.com header.b="Tt2wMg3b";
	dkim=pass (2048-bit key; unprotected) header.d=messagingengine.com header.i=@messagingengine.com header.b="kVONgiOo";
	dkim-atps=neutral
From: <alice@fastmail.com>
To: bot <alice@mail.de>
