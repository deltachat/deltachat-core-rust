Authentication-Results: mx.infomaniak.com; dmarc=pass (p=none dis=none) header.from=hotmail.com
Authentication-Results: mx.infomaniak.com;
	dkim=pass (2048-bit key; unprotected) header.d=hotmail.com header.i=@hotmail.com header.b="Dbq+lYiV";
	dkim-atps=neutral
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@ik.me>
Authentication-Results: mx.infomaniak.com; spf=pass smtp.mailfrom=hotmail.com
