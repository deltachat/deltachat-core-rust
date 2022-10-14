Authentication-Results: mail.buzon.uy;
	dkim=pass (2048-bit key; unprotected) header.d=hotmail.com header.i=@hotmail.com header.b="dEHn9Szj";
	dkim-atps=neutral
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@buzon.uy>
