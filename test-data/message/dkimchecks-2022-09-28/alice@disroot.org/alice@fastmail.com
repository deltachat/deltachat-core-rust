Authentication-Results: disroot.org;
	dkim=pass (2048-bit key; unprotected) header.d=fastmail.com header.i=@fastmail.com header.b="OFgq9UWZ";
	dkim=pass (2048-bit key; unprotected) header.d=messagingengine.com header.i=@messagingengine.com header.b="B52d7C0G";
	dkim-atps=neutral
From: <alice@fastmail.com>
To: <alice@disroot.org>
