Authentication-Results: mail.buzon.uy;
	dkim=pass (2048-bit key; unprotected) header.d=yahoo.com header.i=@yahoo.com header.b="a1T2PpDI";
	dkim-atps=neutral
From: <alice@yahoo.com>
To: <alice@buzon.uy>
