Authentication-Results: mx.infomaniak.com; dmarc=pass (p=none dis=none) header.from=outlook.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (2048-bit key; unprotected) header.d=outlook.com header.i=@outlook.com header.b="fGclt/Vk";
	dkim-atps=neutral
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=outlook.com
