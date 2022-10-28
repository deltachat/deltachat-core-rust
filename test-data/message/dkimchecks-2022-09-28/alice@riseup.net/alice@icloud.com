Authentication-Results: mx1.riseup.net;
	dkim=pass (2048-bit key; unprotected) header.d=icloud.com header.i=@icloud.com header.a=rsa-sha256 header.s=1a1hai header.b=wns6PtC+;
	dkim-atps=neutral
From: <alice@icloud.com>
To: bot <alice@riseup.net>
