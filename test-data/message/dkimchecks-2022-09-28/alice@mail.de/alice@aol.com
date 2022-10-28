Authentication-Results: mxpostfix01.mail.de;
	dkim=pass (2048-bit key; unprotected) header.d=aol.com header.i=@aol.com header.b="cMT1rpDE";
	dkim-atps=neutral
From: <alice@aol.com>
To: bot <alice@mail.de>
