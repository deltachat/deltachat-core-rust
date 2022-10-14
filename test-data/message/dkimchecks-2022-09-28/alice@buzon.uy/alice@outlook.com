Authentication-Results: mail.buzon.uy;
	dkim=pass (2048-bit key; unprotected) header.d=outlook.com header.i=@outlook.com header.b="Uq5LH/n/";
	dkim-atps=neutral
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@buzon.uy>
