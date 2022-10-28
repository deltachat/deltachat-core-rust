ARC-Authentication-Results: i=2; mx2.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=fail smtp.helo=EUR05-DB8-obe.outbound.protection.outlook.com
    policy.ptr=mail-db8eur05olkn2101.outbound.protection.outlook.com;
    bimi=skipped (DMARC Policy is not at enforcement);
    arc=pass (as.1.microsoft.com=pass, ams.1.microsoft.com=pass)
    smtp.remote-ip=40.92.89.101;
    dkim=pass (2048-bit rsa key sha256) header.d=hotmail.com
    header.i=@hotmail.com header.b=FbLQTic7 header.a=rsa-sha256
    header.s=selector1 x-bits=2048;
    dmarc=pass policy.published-domain-policy=none
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=none,d=none,d.eval=none) policy.policy-from=p
    header.from=hotmail.com;
    iprev=pass smtp.remote-ip=40.92.89.101
    (mail-db8eur05olkn2101.outbound.protection.outlook.com);
    spf=pass smtp.mailfrom=alice@hotmail.com
    smtp.helo=EUR05-DB8-obe.outbound.protection.outlook.com
Authentication-Results: mx2.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=fail smtp.helo=EUR05-DB8-obe.outbound.protection.outlook.com
      policy.ptr=mail-db8eur05olkn2101.outbound.protection.outlook.com
Authentication-Results: mx2.messagingengine.com;
    bimi=skipped (DMARC Policy is not at enforcement)
Authentication-Results: mx2.messagingengine.com;
    arc=pass (as.1.microsoft.com=pass, ams.1.microsoft.com=pass)
      smtp.remote-ip=40.92.89.101
Authentication-Results: mx2.messagingengine.com;
    dkim=pass (2048-bit rsa key sha256) header.d=hotmail.com
      header.i=@hotmail.com header.b=FbLQTic7 header.a=rsa-sha256
      header.s=selector1 x-bits=2048;
    dmarc=pass policy.published-domain-policy=none
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=none,d=none,d.eval=none) policy.policy-from=p
      header.from=hotmail.com;
    iprev=pass smtp.remote-ip=40.92.89.101
      (mail-db8eur05olkn2101.outbound.protection.outlook.com);
    spf=pass smtp.mailfrom=alice@hotmail.com
      smtp.helo=EUR05-DB8-obe.outbound.protection.outlook.com
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@fastmail.com>
