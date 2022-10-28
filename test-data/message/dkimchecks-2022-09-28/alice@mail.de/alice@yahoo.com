Authentication-Results: mxpostfix01.mail.de;
	dkim=pass (2048-bit key; unprotected) header.d=yahoo.com header.i=@yahoo.com header.b="si+QZzxa";
	dkim-atps=neutral
From: <alice@yahoo.com>
To: bot <alice@mail.de>
