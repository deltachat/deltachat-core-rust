Authentication-Results: mx.infomaniak.com; dmarc=pass (p=reject dis=none) header.from=yahoo.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (2048-bit key; unprotected) header.d=yahoo.com header.i=@yahoo.com header.b="sJ+4wNJ7";
	dkim-atps=neutral
From: <alice@yahoo.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=yahoo.com
