Authentication-Results: mxpostfix03.mail.de;
	dkim=pass (2048-bit key; unprotected) header.d=outlook.com header.i=@outlook.com header.b="lZ/3SL2c";
	dkim-atps=neutral
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: bot <alice@mail.de>
