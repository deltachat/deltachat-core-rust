Authentication-Results: mx.infomaniak.com; dmarc=pass (p=reject dis=none) header.from=aol.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (2048-bit key; unprotected) header.d=aol.com header.i=@aol.com header.b="Txpx5K4S";
	dkim-atps=neutral
From: <alice@aol.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=aol.com
