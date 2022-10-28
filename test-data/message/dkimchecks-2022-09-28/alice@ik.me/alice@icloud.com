Authentication-Results: mx.infomaniak.com; dmarc=pass (p=quarantine dis=none) header.from=icloud.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (2048-bit key; unprotected) header.d=icloud.com header.i=@icloud.com header.b="RYsH+EvP";
	dkim-atps=neutral
From: <alice@icloud.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=icloud.com
